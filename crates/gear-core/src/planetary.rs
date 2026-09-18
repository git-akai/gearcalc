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
//! `docs/reference.md#internal-gears` records what happened to the attempt that treated it as
//! one. Efficiency is docs/reference.md#planetary-sets and belongs with the stage.

use crate::mesh::{operating_geometry, MeshKind};
use crate::plane::BasicRack;
use crate::solve::{newton_bracketed, Tol};

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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Layout {
    /// The three thickness shifts as the set actually runs, sun, planet, ring —
    /// [`Set::shift`] with the absorber's entry filled in by the solve.
    pub shift: [f64; 3],
    /// The common centre distance, mm — sun-to-planet and planet-to-ring, which
    /// are now the same number: the one the set **runs** at, with
    /// [`Set::clearance`] in both meshes.
    pub centre_distance: f64,
    /// The two meshes' zero-backlash distances, mm, sun–planet then
    /// planet–ring. They differ by twice the clearance — the external mesh runs
    /// a clearance *above* its own and the internal one a clearance *below*
    /// ([`MeshKind::run_at`]) — and are equal only at none.
    pub nominal: [f64; 2],
    /// Operating pressure angle of the sun–planet mesh, radians.
    pub alpha_w_sun: f64,
    /// ...and of the planet–ring mesh, radians.
    pub alpha_w_ring: f64,
    /// Residual between the two running distances at the returned shift, mm.
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
    pub rack: BasicRack,
    pub teeth: Teeth,
    /// How many planets. One is legal — it has no neighbour to clear.
    pub planets: u32,
    /// Thickness shifts, `x + x_s`, in the order sun, planet, ring — the ring's
    /// acting on its **space** (see [`crate::ring::Ring`]).
    ///
    /// The [`Self::absorber`]'s entry is not read: it is what the solve is for.
    pub shift: [f64; 3],
    /// **Which shift closes the set.**
    ///
    /// The two centre distances have to agree, and that is one equation among
    /// three shifts — so two are a design and the third is whatever they leave.
    /// Which one that is belongs to the caller, not here.
    pub absorber: Member,
    /// **The nominal centre distance to hit**, where a designer gave one.
    ///
    /// Without it the set has one equation — the two distances must agree — and
    /// whatever common value they agree at is the answer. With it there are
    /// **two**: each mesh must reach this distance, and each of those is
    /// [`crate::mesh::shift_sum_for`] in closed form. So a target does not make
    /// the solve harder; it makes it easier, and the iteration disappears.
    ///
    /// The count of absorbed shifts follows the count of equations: one without
    /// a target, two with. [`Self::absorber`] names the freedom that is *kept*
    /// in that case rather than the one that gives way — the planet, being the
    /// member in both meshes, is the coordinate the other two are read off.
    pub distance: Option<f64>,
    /// The planet's tip diameter, mm — needed only for planet-to-planet
    /// clearance, which is the one check that cares how big a planet is rather
    /// than how many teeth it has.
    pub planet_tip_diameter: f64,
    /// **The running clearance, in both meshes**, mm.
    ///
    /// One physical distance carries two meshes, and a clearance opens them in
    /// opposite directions: the sun–planet pair parts as the planet moves out,
    /// the planet–ring pair as it moves in. So the two zero-backlash distances
    /// cannot be the same number — they must differ by `2c`, and that is what
    /// the shifts are solved to leave. Written as a distance the two agreed at
    /// with the clearance added afterwards, the internal mesh ran a clearance
    /// *tighter* than zero backlash, and the shifts never heard of it
    /// (`docs/corrections.md`).
    pub clearance: f64,
}

impl Set {
    /// The zero-backlash distance each mesh must have to run at `running` with
    /// this set's clearance — sun–planet, then planet–ring.
    #[must_use]
    pub fn nominal_at(&self, running: f64) -> [f64; 2] {
        [
            MeshKind::External.nominal_of(running, self.clearance),
            MeshKind::Internal.nominal_of(running, self.clearance),
        ]
    }
}

/// One of the three members, and so one of [`Set::shift`]'s entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Member {
    Sun = 0,
    Planet = 1,
    Ring = 2,
}

impl Member {
    /// Its place in [`Set::shift`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
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
    let lo = -reach * sum_ext - set.shift[Member::Sun.index()];
    // `sum_int` is negative, so this is the *upper* bound; writing it with the
    // signed sum keeps it the same expression as the line above rather than a
    // mirrored one.
    let hi = set.shift[Member::Ring.index()] - reach * sum_int;
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

/// Solve for the shift that makes the two centre distances equal.
///
/// # Two of the three are a design and the third is what they leave
///
/// One equation, three shifts. Which one closes it is [`Set::absorber`]'s, and
/// the three cases are not equally hard — which is the whole reason to say so
/// rather than to solve them all the same way. The planet is in **both**
/// meshes, so its shift moves both centre distances at once and the residual
/// has to be driven to zero numerically. The sun is in one mesh only, and so is
/// the ring: fix the other two shifts and the mesh the absorber is *not* in
/// gives the distance outright, leaving the absorber's own mesh a shift sum to
/// hit at a known distance. That is [`crate::mesh::shift_sum_for`] — the same
/// relation a spur pair reads a given centre distance through, and a closed
/// form rather than an iteration.
///
/// # Errors
///
/// `None` when no such shift serves — either the involute domain admits none
/// ([`shift_bracket`]), or the residual does not change sign across it, which
/// for a strictly increasing `g` means the root lies outside the domain. A
/// tooth count that cannot work is the common case, not an exceptional one:
/// most `z_ring` values fail here.
#[must_use]
pub fn solve(set: &Set) -> Option<Layout> {
    let (rack, teeth) = (&set.rack, set.teeth);
    let (sum_ext, sum_int) = teeth.sums();
    if teeth.ring <= teeth.planet || teeth.planet == 0 || teeth.sun == 0 {
        return None;
    }
    let (sun_shift, ring_shift) = (
        set.shift[Member::Sun.index()],
        set.shift[Member::Ring.index()],
    );

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

    // Each mesh where it runs — the external one a clearance above its own
    // zero-backlash distance, the internal one a clearance below — and the set
    // closes where those two are one number.
    let runs = |a_e: f64, a_i: f64| {
        (
            MeshKind::External.run_at(a_e, set.clearance),
            MeshKind::Internal.run_at(a_i, set.clearance),
        )
    };
    let g = |x_p: f64| match (ext(x_p), int(x_p)) {
        (Some((_, _, a_e)), Some((_, _, a_i))) => {
            let (e, i) = runs(a_e, a_i);
            e - i
        }
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

    // The absorber, and with it the three shifts the set actually runs at.
    let sum_x_for = |sum_z: f64, a_w: f64| {
        crate::mesh::shift_sum_for(rack.mt, rack.alpha_t, rack.alpha_n, sum_z, a_w)
    };
    // **A given distance is two equations, and both are closed form.** Each
    // mesh has a shift sum it must reach to run at that distance with the
    // clearance, so the two sums are known outright:
    //
    //     x_s + x_p = shift_sum_for(sum_ext, a − c)
    //     x_p − x_r = shift_sum_for(sum_int, a + c)
    //
    // which leaves **one** freedom. It is taken as the planet's shift, the
    // member in both meshes, and the other two are read off it — so a target
    // removes the Newton iteration below rather than adding to it.
    if let Some(target) = set.distance {
        let [n_ext, n_int] = set.nominal_at(target);
        let s_ext = sum_x_for(sum_ext, n_ext)?;
        let s_int = sum_x_for(sum_int, n_int)?;
        let x_p = set.shift[Member::Planet.index()];
        let shift = [s_ext - x_p, x_p, x_p - s_int];
        return finish(set, shift);
    }

    let shift: [f64; 3] = match set.absorber {
        Member::Planet => {
            let (lo, hi) = shift_bracket(set)?;
            // The endpoints are on the domain boundary, so bring each just
            // inside before handing them over — see `pull_in`.
            let mid = 0.5 * (lo + hi);
            if !g(mid).is_finite() {
                return None;
            }
            let (lo, hi) = (pull_in(&g, lo, mid)?, pull_in(&g, hi, mid)?);
            // Newton from zero shift, which is where the answer sits for the
            // ideal ring and near it for its neighbours; the maintained bracket
            // makes the seed a convenience rather than a requirement.
            let x_p = newton_bracketed(g, dg, lo, hi, 0.0_f64.clamp(lo, hi), Tol::default())?;
            [sun_shift, x_p, ring_shift]
        }
        // The planet's shift is given, so the mesh the absorber is not in fixes
        // the running distance and its own mesh has a shift sum to reach at it.
        Member::Sun => {
            let x_p = set.shift[Member::Planet.index()];
            let (_, _, a_i) = int(x_p)?;
            let [n_ext, _] = set.nominal_at(MeshKind::Internal.run_at(a_i, set.clearance));
            [sum_x_for(sum_ext, n_ext)? - x_p, x_p, ring_shift]
        }
        Member::Ring => {
            let x_p = set.shift[Member::Planet.index()];
            let (_, _, a_e) = ext(x_p)?;
            let [_, n_int] = set.nominal_at(MeshKind::External.run_at(a_e, set.clearance));
            [sun_shift, x_p, x_p - sum_x_for(sum_int, n_int)?]
        }
    };

    finish(set, shift)
}

/// **Read back from the three shifts, however they were arrived at.**
///
/// The two distances are computed again rather than carried out of whichever
/// branch produced the shifts, so `residual` measures the answer being returned
/// rather than an intermediate that branch happened to hold. It is also what
/// makes a given distance and a solved agreement the same kind of answer: both
/// arrive here with three numbers and are judged the same way.
fn finish(set: &Set, shift: [f64; 3]) -> Option<Layout> {
    let (rack, teeth, planets) = (&set.rack, set.teeth, set.planets);
    let (sum_ext, sum_int) = teeth.sums();
    let (x_s, x_p, x_r) = (shift[0], shift[1], shift[2]);
    let (alpha_w_sun, _, a_e) =
        operating_geometry(rack.mt, rack.alpha_t, rack.alpha_n, sum_ext, x_s + x_p)?;
    let (alpha_w_ring, _, a_i) =
        operating_geometry(rack.mt, rack.alpha_t, rack.alpha_n, sum_int, x_p - x_r)?;

    let running = MeshKind::External.run_at(a_e, set.clearance);
    let residual = (running - MeshKind::Internal.run_at(a_i, set.clearance)).abs();

    let equal_spacing = planets > 0 && (teeth.sun + teeth.ring) % planets == 0;
    let simultaneous_meshing = planets > 0 && teeth.sun % planets == 0 && teeth.ring % planets == 0;
    // Where the planets actually sit, which is the running distance.
    let planet_clearance = (planets > 1).then(|| {
        2.0 * running * (std::f64::consts::PI / f64::from(planets)).sin() - set.planet_tip_diameter
    });

    Some(Layout {
        shift,
        centre_distance: running,
        nominal: [a_e, a_i],
        alpha_w_sun,
        alpha_w_ring,
        residual,
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
    // **The planet absorbs here whatever the caller's own set does**, because
    // the completeness argument above is about the planet's shift rising with
    // the ring's count. Sweeping a set whose sun or ring closes it would be
    // sweeping the very shift the sweep varies against.
    for ring in (set.teeth.planet + 1)..=limit {
        let candidate = Set {
            teeth: Teeth { ring, ..set.teeth },
            absorber: Member::Planet,
            ..*set
        };
        let Some(layout) = solve(&candidate) else {
            continue;
        };
        let needed = layout.shift[Member::Planet.index()];
        if needed < shift_range.0 {
            continue;
        }
        // Monotone in `z_ring`, so once the required shift passes the top of the
        // range every larger ring does too.
        if needed > shift_range.1 {
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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PlanetaryShaft {
    Sun,
    Carrier,
    Ring,
}

impl PlanetaryShaft {
    /// The three shafts, in the order the arrays below index them.
    ///
    /// Named once because it was written out three times — in `other`, in the
    /// tests' arrangement sweep and in the harness — and a list of three
    /// written four times is a list that can come to be three.
    pub const ALL: [Self; 3] = [Self::Sun, Self::Carrier, Self::Ring];

    /// Index into the `[sun, carrier, ring]` arrays below.
    const fn index(self) -> usize {
        match self {
            Self::Sun => 0,
            Self::Carrier => 1,
            Self::Ring => 2,
        }
    }

    /// Index into the `[sun, carrier, ring]` arrays, for callers outside this
    /// module that hold those arrays.
    #[must_use]
    pub const fn index_pub(self) -> usize {
        self.index()
    }

    /// The member that is neither of these two.
    ///
    /// A planetary set has three shafts and exactly two are chosen — one driven,
    /// one held — so the third is not a choice at all.
    pub(crate) fn other(a: Self, b: Self) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|&m| m != a && m != b)
            .filter(|_| a != b)
    }
}

/// Which shaft drives and which is held.
///
/// **This is an addition to the specification's field list** (docs/rationale.md#additions-to-the-specifications-field-list). The
/// specification names only "Driven By", which picks one shaft of three and
/// leaves the arrangement undetermined: a sun-driven set behaves quite
/// differently with the ring held than with the carrier held. Naming the held
/// shaft as well is what makes the six modes of docs/reference.md#planetary-sets reachable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Arrangement {
    pub input: PlanetaryShaft,
    pub fixed: PlanetaryShaft,
}

/// What the three shafts do, and what it costs to make them do it.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
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
    pub output: PlanetaryShaft,
    /// Sign of the rolling power — which way power crosses the meshes in the
    /// carrier's frame. `+1` when the sun leads the carrier under a driving
    /// torque, `−1` when it trails.
    pub rolling_power_sign: f64,
}

impl Power {
    /// **The planet's own rotation**, absolute and relative to the carrier.
    ///
    /// **An oracle, not the model.** Nothing shipped reads this any more: a
    /// planet's speed comes off the shaft-line graph with every other member's
    /// (`train::Wiring::unit_motion`). It is kept, test-only, because it is a
    /// *second derivation* — Willis on the sun mesh in the carrier's frame —
    /// and the graph is held against it in `kinematics::tests`. That is the
    /// plan's sequencing done as it said: the old model becomes the fixture
    /// rather than being deleted with nothing to stand in its place.
    ///
    /// The planet is not one of the three shafts — [`Power`] is about a set with
    /// a basic ratio, and a basic ratio does not say how many teeth the thing
    /// between the two central members has — so its speed is a question for the
    /// tooth counts, asked here where the rest of the kinematics live.
    ///
    /// In the carrier's frame the sun mesh is an ordinary external pair:
    ///
    /// ```text
    /// ω_p − ω_c = −(z_s / z_p) (ω_s − ω_c)
    /// ```
    ///
    /// The ring mesh gives the same number by the other road,
    /// `+(z_r / z_p)(ω_r − ω_c)`, which is Willis again and is what
    /// `the_planets_own_speed_is_what_both_of_its_meshes_say` checks.
    ///
    /// **The relative figure is the one its teeth see**, and it is not the sun's
    /// relative speed: the two differ by `z_s/z_p`, which is a third on an
    /// ordinary set. Nor is its absolute speed the carrier's — the planet spins
    /// on the arm as well as riding it, and on a set with the ring held the two
    /// have opposite signs.
    ///
    /// Returns `(absolute, relative to the carrier)`, in the unit the speeds
    /// were given in.
    #[cfg(test)]
    #[must_use]
    pub fn planet_speed(&self, teeth: Teeth) -> (f64, f64) {
        let carrier = self.speeds[PlanetaryShaft::Carrier.index_pub()];
        let sun = self.speeds[PlanetaryShaft::Sun.index_pub()];
        let relative = -(f64::from(teeth.sun) / f64::from(teeth.planet)) * (sun - carrier);
        (carrier + relative, relative)
    }
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
/// # What it is about
///
/// **Three shafts, a basic ratio and a fixed-carrier efficiency** — and nothing
/// else. Sun, carrier and ring name the three *roles*: the two central members
/// on the common axis, and the arm that carries whatever runs between them.
/// Which tooth form each central member has, and what the planet is, reach this
/// solve only through the basic ratio it is handed. So any three-shaft
/// epicyclic can be put through it by naming its own members in those roles —
/// which is why the ratio arrives as a number rather than as a set of planetary
/// tooth counts to derive one from.
///
/// # The method
///
/// Pennestrì–Freudenstein, as docs/reference.md#planetary-sets sets it out. Two linear relations carry
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
/// `None` when the same member is both driven and held, when the named input is
/// not actually driving (`T ω ≤ 0` — see above), when the tooth counts make a
/// relation degenerate, or when neither sign of the rolling power is
/// self-consistent.
#[must_use]
pub fn power(
    basic_ratio: f64,
    arrangement: Arrangement,
    input_speed: f64,
    input_torque: f64,
    fixed_carrier_efficiency: f64,
) -> Option<Power> {
    let output = PlanetaryShaft::other(arrangement.input, arrangement.fixed)?;
    let (i, f, o) = (
        arrangement.input.index(),
        arrangement.fixed.index(),
        output.index(),
    );
    let i0 = basic_ratio;
    if !i0.is_finite() || !fixed_carrier_efficiency.is_finite() {
        return None;
    }
    // **The named input has to be driving.** A shaft with `T ω < 0` is absorbing
    // power, so calling it the input is a contradiction rather than a design —
    // and the arithmetic says so plainly, returning `1/η₀ > 1`. Refused, because
    // the fix is to name the shaft that is actually driving.
    let input_power = input_torque * input_speed;
    if !input_power.is_finite() || input_power <= 0.0 {
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
        // **...and the output has to absorb what the input delivers.**
        //
        // Self-consistency in the rolling sign is necessary and not sufficient.
        // `k = i₀ η₀^w` sits either side of 1 as `w` flips, and where `i₀` is
        // itself close to 1 — which is exactly what a set reducing by the square
        // of a tooth count is — the two candidates straddle it. Then `1 − k`
        // changes sign between the branches, so the sun's torque does, so the
        // rolling power does, and **both branches confirm their own assumption**.
        //
        // What separates them is where the power goes. One puts the output's
        // torque along its own rotation, which is a shaft *delivering* power
        // while the input delivers too and friction makes up the difference —
        // energy from nowhere, and it shows up as an efficiency above 1. The
        // other has the output absorbing, the loss positive, and an efficiency
        // below it. On a well conditioned set only one branch was ever
        // self-consistent and this changes nothing; near `i₀ = 1` it is the
        // whole answer.
        if torques[o] * speeds[o] > 0.0 {
            continue;
        }
        let efficiency = (torques[o] * speeds[o]).abs() / input_power;
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
        let all = [
            PlanetaryShaft::Sun,
            PlanetaryShaft::Carrier,
            PlanetaryShaft::Ring,
        ];
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

    /// **A genuinely driving input always has a flow**, and that is why the
    /// refusal a set can meet is the *back-driven* one.
    ///
    /// [`power`] returns `None` three ways: a degenerate Willis coefficient, an
    /// input that is not driving (`T ω ≤ 0`), and neither sign of the rolling
    /// power confirming itself. A stage asks it twice — forward at unit speed
    /// and unit torque, and backward with the output's own reaction — and only
    /// the second can refuse, which is self-locking and is an *answer*.
    ///
    /// This is the standing evidence for that, and for
    /// `gear_io::strings::UNFIRED`'s exemption of `error.train_no_power_flow`:
    /// **an absence has a date**, and a note claiming one is worth less than a
    /// sweep that fails if it stops being true. Sun against ring across the
    /// whole plausible range, `η₀` from near-lossless down to 0.3 — far below
    /// anything two involute meshes produce — and every arrangement.
    #[test]
    fn a_driving_input_always_has_a_flow() {
        let mut checked = 0u32;
        for zs in (1..=119).step_by(2) {
            for zr in (1..=249).step_by(3) {
                let i0 = -f64::from(zr) / f64::from(zs);
                for eta0 in [0.999, 0.97, 0.9, 0.7, 0.5, 0.3] {
                    for arrangement in arrangements() {
                        assert!(
                            power(i0, arrangement, 1.0, 1.0, eta0).is_some(),
                            "z {zs}/{zr}, eta0 {eta0}, {arrangement:?} refused a driving input"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100_000, "only {checked} combinations swept");

        // ...and the two refusals that *are* reachable, so this is a statement
        // about driving inputs rather than about `power` never saying no.
        let ordinary = Arrangement {
            input: PlanetaryShaft::Sun,
            fixed: PlanetaryShaft::Carrier,
        };
        assert!(
            power(basic_ratio(teeth()), ordinary, 1.0, -1.0, 0.97).is_none(),
            "a shaft with T omega < 0 is not an input"
        );
        assert!(
            power(0.0, ordinary, 1.0, 1.0, 0.97).is_none(),
            "a zero basic ratio leaves the ring with no Willis coefficient"
        );
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
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Carrier);
        assert!((p.ratio - (1.0 + zr / zs)).abs() < 1e-12, "{}", p.ratio);

        // Sun held, ring driving: 1 + z_s/z_r.
        let p = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Ring,
                fixed: PlanetaryShaft::Sun,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Carrier);
        assert!((p.ratio - (1.0 + zs / zr)).abs() < 1e-12, "{}", p.ratio);

        // Carrier held: the sun and ring turn opposite ways, ratio −z_r/z_s.
        let p = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Carrier,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Ring);
        assert!((p.ratio - (-zr / zs)).abs() < 1e-12, "{}", p.ratio);
        assert!(p.ratio < 0.0, "a fixed carrier reverses the output");
    }

    /// A set built from lossless meshes is lossless, in **all six** arrangements
    /// and exactly — so the loss term enters only through `η₀` and nothing else
    /// leaks.
    /// **Self-consistency in the rolling sign does not pick the branch on its
    /// own**, and where it cannot, energy is what decides.
    ///
    /// `k = i₀ η₀^w` sits either side of 1 as `w` flips, so where `i₀` is itself
    /// close to 1 the two candidates straddle it, `1 − k` changes sign between
    /// them, and both confirm their own assumption. One of the two has the
    /// output's torque along its own rotation — a shaft delivering power while
    /// the input delivers too — which is energy from nowhere and shows up as an
    /// efficiency above 1. Taking the first self-consistent branch returned
    /// exactly that: 1.22 at `i₀ = 144/143`.
    ///
    /// This is the regime of a set that reduces by the square of a tooth count,
    /// so it is ordinary rather than pathological — and nothing about the
    /// arrangement warns of it, which is why the guard is on the physics.
    #[test]
    fn a_basic_ratio_near_one_still_loses_power() {
        for (num, den) in [(144_i64, 143_i64), (324, 323), (900, 899), (2500, 2499)] {
            let i0 = num as f64 / den as f64;
            for eta0 in [0.98, 0.99, 0.995] {
                let p = power(
                    i0,
                    Arrangement {
                        input: PlanetaryShaft::Carrier,
                        fixed: PlanetaryShaft::Sun,
                    },
                    1000.0,
                    2.0,
                    eta0,
                )
                .unwrap_or_else(|| panic!("i0 {i0} eta0 {eta0} should solve"));
                assert!(
                    p.efficiency > 0.0 && p.efficiency < 1.0,
                    "i0 {i0} eta0 {eta0}: efficiency {} is not one",
                    p.efficiency
                );
                let out = PlanetaryShaft::Ring.index_pub();
                assert!(
                    p.torques[out] * p.speeds[out] <= 0.0,
                    "i0 {i0}: the output delivers power as well as the input"
                );
            }
        }
    }

    /// **The planet's own speed is what both of its meshes say.**
    ///
    /// [`Power::planet_speed`] reads it off the sun mesh in the carrier's frame;
    /// this rebuilds it from the *ring* mesh, `+(z_r/z_p)(ω_r − ω_c)`. Different
    /// pair, different tooth counts, and the two agree only if the relative
    /// speed is genuinely the planet's rather than the sun's — which is the
    /// mistake this gates, and the one the stage made: it reported the sun's
    /// relative speed as the planet's, a third out on an ordinary set, and the
    /// carrier's absolute speed as the planet's, which on a set with the ring
    /// held is not even the same sign.
    #[test]
    fn the_planets_own_speed_is_what_both_of_its_meshes_say() {
        for teeth in [
            Teeth {
                sun: 24,
                planet: 18,
                ring: 60,
            },
            Teeth {
                sun: 17,
                planet: 17,
                ring: 51,
            },
            Teeth {
                sun: 40,
                planet: 10,
                ring: 60,
            },
        ] {
            for fixed in [
                PlanetaryShaft::Ring,
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
            ] {
                let input = match fixed {
                    PlanetaryShaft::Ring | PlanetaryShaft::Carrier => PlanetaryShaft::Sun,
                    PlanetaryShaft::Sun => PlanetaryShaft::Carrier,
                };
                let p = power(
                    basic_ratio(teeth),
                    Arrangement { input, fixed },
                    3000.0,
                    2.0,
                    1.0,
                )
                .unwrap_or_else(|| panic!("{teeth:?} held at {fixed:?} should solve"));
                let carrier = p.speeds[PlanetaryShaft::Carrier.index_pub()];
                let ring = p.speeds[PlanetaryShaft::Ring.index_pub()];
                let want = (f64::from(teeth.ring) / f64::from(teeth.planet)) * (ring - carrier);
                let (absolute, relative) = p.planet_speed(teeth);
                assert!(
                    (relative - want).abs() < 1e-9 * want.abs().max(1.0),
                    "{teeth:?} held at {fixed:?}: the sun mesh gives {relative}, the ring {want}"
                );
                assert!(
                    (absolute - (carrier + relative)).abs() < 1e-9,
                    "the absolute speed is the carrier's plus its own spin"
                );
            }
        }
    }

    /// A set that loses more than half of what it is given cannot be driven
    /// backwards: the reversed flow has no branch where the output absorbs, and
    /// the classical `2 − 1/η` for such a set is negative.
    #[test]
    fn a_set_below_half_efficiency_does_not_back_drive() {
        let i0 = 324.0 / 323.0;
        let fixed = PlanetaryShaft::Sun;
        let forward = power(
            i0,
            Arrangement {
                input: PlanetaryShaft::Carrier,
                fixed,
            },
            1000.0,
            2.0,
            0.99,
        )
        .unwrap();
        assert!(forward.efficiency < 0.5, "{}", forward.efficiency);
        let out = PlanetaryShaft::Ring.index_pub();
        let back = power(
            i0,
            Arrangement {
                input: PlanetaryShaft::Ring,
                fixed,
            },
            forward.speeds[out],
            forward.torques[out].abs(),
            0.99,
        );
        assert!(
            back.is_none(),
            "back-driving should have no state at all, not {:?}",
            back.map(|p| p.efficiency)
        );
        assert!(
            2.0 - 1.0 / forward.efficiency < 0.0,
            "and the relation agrees"
        );
    }

    #[test]
    fn a_lossless_set_is_lossless_in_every_arrangement() {
        for a in arrangements() {
            let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, 1.0).unwrap();
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
                let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, eta).unwrap();
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
            let ideal = power(basic_ratio(teeth()), a, 1500.0, 3.0, 1.0).unwrap();
            let mut last = ideal.efficiency;
            for eta in [0.99, 0.97, 0.94, 0.9] {
                let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, eta).unwrap();
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
            for input in [PlanetaryShaft::Sun, PlanetaryShaft::Ring] {
                let p = power(
                    basic_ratio(teeth()),
                    Arrangement {
                        input,
                        fixed: PlanetaryShaft::Carrier,
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
                basic_ratio(t),
                Arrangement {
                    input: PlanetaryShaft::Sun,
                    fixed: PlanetaryShaft::Ring,
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

    /// **Efficiency never exceeds one in either drive sense** — the test that was
    /// missing, and that a 101.571 % figure in the running application found.
    ///
    /// Reversing a set means the shaft that was the output now drives, so its
    /// torque must have the same sign as its speed. Hand it the *reaction* torque
    /// the forward solution left there and the rolling power comes out the wrong
    /// way round, `η₀^w` takes the wrong branch, and the answer is above unity.
    /// Every earlier test drove forward with a positive torque and a positive
    /// speed, so none of them could see it.
    #[test]
    fn no_arrangement_is_efficient_above_one_in_either_direction() {
        for a in arrangements() {
            for eta0 in [1.0, 0.99, 0.97, 0.9] {
                // Both senses of a genuinely driving input.
                for (speed, torque) in [(1500.0, 3.0), (-1500.0, -3.0)] {
                    let p = power(basic_ratio(teeth()), a, speed, torque, eta0).unwrap();
                    assert!(
                        p.efficiency <= 1.0 + 1e-12,
                        "{a:?} eta0={eta0} n={speed} T={torque}: efficiency {}",
                        p.efficiency
                    );
                    if eta0 < 1.0 {
                        assert!(p.efficiency < 1.0, "{a:?} eta0={eta0}: lossless at a loss");
                    }
                }
                // ...and a shaft that absorbs power is not an input. Asking
                // anyway used to return `1/η₀`, above one, which is the
                // arithmetic saying the question was put the wrong way round.
                assert!(
                    power(basic_ratio(teeth()), a, 1500.0, -3.0, eta0).is_none(),
                    "{a:?}: a non-driving input must be refused"
                );
            }
        }
    }

    /// **A coupled planetary is not as efficient as its meshes.**
    ///
    /// The result worth surfacing, and the reason docs/reference.md#planetary-sets refuses a mesh-by-mesh
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
                basic_ratio(t),
                Arrangement {
                    input: PlanetaryShaft::Carrier,
                    fixed: PlanetaryShaft::Ring,
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
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
            },
            1000.0,
            5.0,
            eta0,
        )
        .unwrap();
        let ring_out = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Carrier,
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
            let p = power(basic_ratio(teeth()), a, 1234.0, 7.0, 0.98).unwrap();
            assert_eq!(p.speeds[a.fixed.index()], 0.0);
            assert_eq!(p.speeds[a.input.index()], 1234.0);
            assert_ne!(p.output, a.input);
            assert_ne!(p.output, a.fixed);
        }
    }

    /// Driving and holding the same shaft is not an arrangement.
    #[test]
    fn a_shaft_cannot_be_both_driven_and_held() {
        for m in [
            PlanetaryShaft::Sun,
            PlanetaryShaft::Carrier,
            PlanetaryShaft::Ring,
        ] {
            assert!(power(
                basic_ratio(teeth()),
                Arrangement { input: m, fixed: m },
                1000.0,
                1.0,
                0.98
            )
            .is_none());
        }
    }

    fn rack() -> BasicRack {
        BasicRack::new(1.0, 20.0, 0.0)
    }

    /// A set with three planets, nothing shifted but the planet, and a planet
    /// small enough that clearance never bites.
    fn set_of(sun: u32, planet: u32, ring: u32) -> Set {
        Set {
            rack: rack(),
            teeth: Teeth { sun, planet, ring },
            planets: 3,
            shift: [0.0; 3],
            absorber: Member::Planet,
            distance: None,
            planet_tip_diameter: 0.0,
            clearance: 0.0,
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
                l.shift[Member::Planet.index()].abs() < 1e-12,
                "z={sun}/{planet}/{ring}: shift {} should be zero",
                l.shift[Member::Planet.index()]
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
                    l.shift[Member::Planet.index()] > last,
                    "z_r={ring}: {} is not above {last}",
                    l.shift[Member::Planet.index()]
                );
                last = l.shift[Member::Planet.index()];
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

    /// The worked example of docs/reference.md#planetary-sets, to the digits recorded there.
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
                (l.shift[Member::Planet.index()] - want).abs() < 5e-5,
                "z_r={ring}: {} vs {want}",
                l.shift[Member::Planet.index()]
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
            assert!((0.0..=0.5).contains(&l.shift[Member::Planet.index()]));
            assert!(l.residual < 1e-12);
        }

        // Which of them a designer wants is a *second* question, and the one
        // docs/reference.md#planetary-sets's worked example answers: only 52 also spaces three planets
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
        let base = solve_at(17, 17, 52).unwrap().shift[Member::Planet.index()];

        // A more positive sun already widens the external mesh, so the planet
        // needs less of its own shift...
        let sunny = solve(&Set {
            shift: [0.2, 0.0, 0.0],
            ..set_of(17, 17, 52)
        })
        .unwrap()
        .shift[Member::Planet.index()];
        assert!(sunny < base, "{sunny} should be below {base}");

        // ...and a more positive ring widens its space, which the planet follows.
        let ringy = solve(&Set {
            shift: [0.0, 0.0, 0.2],
            ..set_of(17, 17, 52)
        })
        .unwrap()
        .shift[Member::Planet.index()];
        assert!(ringy > base, "{ringy} should be above {base}");
    }
}
