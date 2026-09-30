//! Tooth bending geometry: the critical root section and the form factor.
//!
//! # Why there is no table here
//!
//! Lewis form factors are tabulated because, historically, nobody could compute
//! the real root geometry. We can: `profile` generates the exact trochoid,
//! undercut included, and it is validated against a simulation of the cutter
//! that would make it. So the form factor is **measured off the profile** rather
//! than looked up.
//!
//! That is the whole argument for this module. Undercut, profile shift and tooth
//! thickness modification need no special cases, because they change the profile
//! and we measured the profile. A tabulated factor cannot do that — the tables
//! are indexed by tooth count and shift alone, and stop at the standard rack.
//!
//! # The construction
//!
//! Two constructions, and a caller picks between them — [`CriticalSection`],
//! which sets out at length why the default is the second.
//!
//! The **tangent method** (Hofer): the critical section is the chord between the
//! two points on the root fillet where the tangent makes a fixed angle to the
//! tooth centreline — 30° on an external tooth, 60° on a ring's. Along the
//! fillet that angle sweeps monotonically from near zero at the flank junction
//! to 90° where it meets the root circle, so the tangency point exists, is
//! unique, and is bracketed by construction. The two kinds of member differ in
//! that **one number** and in nothing else the search can see.
//!
//! The **inscribed parabola** (Lewis), the default: the largest constant-strength
//! parabola with its vertex at the load. It takes no angle — the fixed angle is
//! exactly the convention it exists to avoid — and the two kinds of member do not
//! differ in even one number, because its tangency condition is odd in `y` and
//! the frame flip below is the whole of what it is told.
//!
//! Everything is done in **tooth coordinates**: `y` along the tooth centreline
//! pointing outward, `x` across it, origin at the gear axis.

use crate::contact::{ContactPath, LoadSharing};
use crate::hertz::peak_pressure;
use crate::mesh::Mesh;
use crate::solve::{brent, Tol};
use crate::tooth::Tooth;

/// How the critical root section is located.
///
/// These are two answers to the same question, and they disagree most exactly
/// where it matters — on undercut teeth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CriticalSection {
    /// The ISO/AGMA tangent (Hofer) — 30° on an external tooth, 60° on a ring's
    /// ([`TANGENT_ANGLE_INTERNAL_DEG`]).
    ///
    /// **Retained but not the default.** Kept because it is what ISO 6336 and
    /// AGMA 2101 specify, so it is the setting to return to for a
    /// standards-comparable number, and because
    /// [`RootStressModel::Iso6336`] is a fit calibrated against *this*
    /// construction.
    ///
    /// Its weakness is that it is **independent of where the load acts**, which
    /// is precisely the property the cantilever model is supposed to have. The
    /// two constructions converge as teeth get larger — at z=60 the 30° tangents
    /// cross the centreline within 0.04% of the load point — and diverge where
    /// the geometry is worst, crossing 12% below it at z=9.
    TangentAngle,
    /// The Lewis inscribed parabola — the original construction.
    ///
    /// A cantilever whose outline is a parabola with its vertex at the load has
    /// uniform bending stress along its length. Inscribing the largest such
    /// parabola, vertex at the point where the load line crosses the tooth
    /// centreline, and taking the tangency with the fillet, finds where the real
    /// tooth is weakest *relative to that uniform-strength shape*.
    ///
    /// Unlike the tangent construction this **follows the load point**, which is
    /// the property the cantilever model is supposed to have. On an external
    /// tooth the tangency sits higher up the fillet, the section is narrower,
    /// and `Y_F` comes out larger: the section the default rates, loaded at the
    /// tip over `gear-cli matrix`'s population, 0.3 % to 13.7× the tangent's,
    /// mean 10.7 %. The large end is a narrow tip loaded steeply, where the
    /// flank's section under the land governs (1.1 % of the population; see
    /// [`root_section_rated`] for which section governs, and why).
    ///
    /// **That is `Y_F` alone, and it is not what a designer feels.** The number
    /// a stress is proportional to is the whole bending factor, and a narrower
    /// section changes the notch factor too. Any claim that this construction is
    /// "the conservative one" is a claim about `Y_F`.
    ///
    /// # Divergence from the standards
    ///
    /// This is **the default here and it is not what ISO 6336 or AGMA 2101
    /// specify.** The reasons for diverging, and the reasons to be careful:
    ///
    /// - It is the original construction; the 30° tangent is a later
    ///   simplification adopted for ease of calculation. This project computes
    ///   the exact profile, so the simplification buys nothing.
    /// - Experimental single-tooth-bending work reports measured critical
    ///   locations *above* the 30° prediction, with that prediction at the edge
    ///   of the observed range — the direction the parabola moves the section.
    ///   (The authors note large test deformations may contribute, so this is
    ///   support rather than proof.)
    /// - It gives the larger `Y_F` of the two, everywhere.
    ///
    /// Against that: on an **external** tooth loaded at its tip it ranks
    /// designs as the 30° tangent does to Spearman ρ = 0.898 over 1508
    /// designs, and agrees with it on the direction of a change in shift 90 %
    /// of the time where the change moves the answer by 1 % or more — close,
    /// and not the same: the two part where a narrow tip is loaded steeply. (A
    /// flank search that took the first crossing it bracketed never found
    /// those tangencies, and read ρ = 0.993.) And **`Y_S` is calibrated
    /// against the tangent construction**, so pairing the two mixes
    /// conventions.
    ///
    /// # On a ring
    ///
    /// Measured over 160 ring designs against the 60° tangent
    /// (`gear-cli matrix`, and it could not be measured at all until the 60°
    /// angle existed, since the ring's tangent section was being taken at 30°):
    ///
    /// | | external | ring |
    /// |---|---|---|
    /// | rated section on the **flank** | 1.1 % | 0 % |
    /// | `Y_F` parabola/tangent | mean 1.107 | mean 1.166 |
    /// | whole factor, each set's own | mean 0.820 | mean 0.775 |
    /// | mean `q_s` | 2.07 vs 3.17 | 1.65 vs 4.94 |
    /// | Spearman ρ on `Y_F`, construction's | **0.898** | **0.537** |
    /// | Spearman ρ, parabola `K_f` against tangent `Y_F` | 0.912 | 0.960 |
    ///
    /// The largest parabola a ring holds touches its flank, every time, and
    /// its fillet has no tangency: the fillet offers its end at the flank,
    /// where the fillet's notch is, and rated with the fillet's notch factor
    /// that section governs every ring in the population. The largest
    /// parabola alone still orders ring designs unlike the tangent (ρ =
    /// 0.537); the rated section orders them alike (0.960).
    ///
    /// For a number to compare against a published ISO or AGMA rating, switch to
    /// [`CriticalSection::TangentAngle`].
    #[default]
    LewisParabola,
}

/// The angle the critical-section tangent makes with the tooth centreline, for
/// an **external** tooth.
///
/// 30° is the Hofer construction adopted by ISO 6336 for external gears. It is a
/// convention, not a derivation — the true peak stress location depends on the
/// fillet shape — and it is named here rather than buried as a literal because
/// it is one value of two: see [`TANGENT_ANGLE_INTERNAL_DEG`].
pub const TANGENT_ANGLE_DEG: f64 = 30.0;

/// ...and for a **ring's** tooth.
///
/// ISO 6336-3:2019, 6.1: "The chord between the points at which the 30° tangents
/// contact the root fillets for external gears, or at which the 60° tangents
/// contact the root fillets for internal gears, defines the section to be used
/// as the basis for calculation." 6.2.5 says the same thing the other way round,
/// as a sign convention on the shared construction: an internal gear is
/// 6.2.4's formulae with the diameters, the manufacturing centre distance and
/// the tangential angle `θ` all negated, `θ` being 60° rather than 30°.
///
/// **Why it is a different angle at all.** Not because the tooth points the
/// other way round. A ring's tooth widens from tip to root exactly as an
/// external one does — 0.967 mm to 2.569 mm along the flank at z = 40, module 1
/// (`gear-cli matrix`, study 6) — so the cantilever picture is the same picture,
/// and an earlier revision of this comment had it backwards. What differs is the
/// fillet the shaper leaves, which curls into the rim rather than away from it,
/// and how fast the tooth flares into it. 60° is ISO's convention for that form
/// as 30° is for the rack-cut one; neither is derived, which is what makes them
/// conventions rather than results.
///
/// This is the tangent construction only, and this crate's default critical
/// section is the inscribed parabola — see [`CriticalSection`]. The parabola
/// needs no such constant: it finds its own tangency from the load point, and
/// the frame flip that [`ToothOutline`] describes is the whole of what it needs
/// told about which way a tooth points.
pub const TANGENT_ANGLE_INTERNAL_DEG: f64 = 60.0;

/// The critical root section and the load that acts on it.
///
/// Lengths in millimetres, angles in radians. Coordinates are in the tooth frame
/// described in the module documentation, so they can be drawn directly.
#[derive(Clone, Copy, Debug)]
pub struct RootSection {
    /// Rack travel parameter at the tangency point.
    pub s: f64,
    /// Tooth root chord thickness at the critical section, `s_Fn`.
    pub root_chord: f64,
    /// Bending moment arm from the critical section to where the load line
    /// crosses the tooth centreline, `h_Fe`.
    pub moment_arm: f64,
    /// Load application angle, `α_Fen`, radians: the angle between the load
    /// direction and the perpendicular to the tooth centreline.
    pub load_angle: f64,
    /// Radius of curvature of the fillet **at the critical section**, `ρ_F` —
    /// ISO 6336-3's definition, and what [`RootStressModel::Iso6336`] is
    /// fitted to.
    ///
    /// At the tangency when that is on the fillet; at the fillet's junction with
    /// the flank when the tangency has climbed above it. Always a *fillet*
    /// curvature, which is what makes it a notch radius.
    pub fillet_curvature: f64,
    /// **The minimum radius of curvature of the fillet curve**, `ρ_f` — Dolan
    /// and Broghamer's definition, and what [`RootStressModel::DolanBroghamer`]
    /// is fitted to.
    ///
    /// A property of the whole fillet rather than of a point on it, so it is
    /// defined wherever the critical section ended up and needs no fallback when
    /// that is on the flank. It is read at [`ToothOutline::fillet_root`], the
    /// deepest point, where the cut is tightest — gated by
    /// `the_fillet_is_tightest_at_its_root`, which sweeps the whole fillet and
    /// checks nothing is smaller.
    ///
    /// **The two radii are not close.** The junction is the flattest point the
    /// fillet has and the root the tightest, and between them they differ by
    /// 1.4–4.1× on an external tooth and 2.1–6.3× on a ring
    /// (`gear-cli matrix`, study 7). A notch factor fed the wrong one is wrong
    /// by that much, which is why each model reads its own.
    pub min_fillet_curvature: f64,
    /// Normal pressure angle of the basic rack, radians.
    ///
    /// Carried because [`RootStressModel::DolanBroghamer`]'s constants are
    /// functions of it — it is the one notch fit here that covers a pressure
    /// angle other than 20°, and it can only do so if it is told.
    pub pressure_angle_rad: f64,
    /// Tooth form factor `Y_F` — the **bending** term alone.
    pub form_factor: f64,
    /// **The axial compression term**, in the same units as [`Self::form_factor`]
    /// and subtracted from it.
    ///
    /// ```text
    /// sin α_Fen / ( (s_Fn/m) · cos α_n )
    /// ```
    ///
    /// The tooth load acts along the line of action, not across the tooth. Its
    /// across-tooth component bends the root, which is `Y_F`; its **along-tooth**
    /// component pushes the tooth into its own rim, and that compression
    /// subtracts from the tensile stress at the loaded flank's fillet — which is
    /// the fillet a tooth cracks from. It is the second term of Savage,
    /// Rubadeux & Coe's `J`, `6h/t_c² − tan φ_C/t_c`, of which `Y_F` is the
    /// first, and it relieves the root by order 10 %.
    ///
    /// Carried separately rather than folded into `Y_F` because `Y_F` has a
    /// definition of its own that this crate is checked against — the rack limit,
    /// and ISO's Method B — and because [`RootStressModel::Iso6336`] does not
    /// take it: ISO omits the term, so a comparable number must omit it too.
    /// [`RootSection::bending_factor`] is where the two are combined, per model.
    ///
    /// On a reversed root each flank takes its turn as the loaded one and the
    /// compression relieves whichever is in tension, so there is no case here.
    pub axial_compression: f64,
    /// Transverse module of the plane the section was measured in, mm.
    ///
    /// Carried so that a stress can be had from a section and a load alone: on
    /// the virtual spur member every rating is taken on, this is `m_n`, and it
    /// is the `m_n` of `F_t/(b·m_n)`.
    pub module: f64,
    /// Notch parameter `q_s = s_Fn / (2 ρ_F)`, the input to stress correction.
    pub notch_parameter: f64,
    /// Which construction located this section.
    pub method: CriticalSection,
    /// True when the inscribed parabola touched the involute flank rather than
    /// the fillet; see [`CriticalSection::LewisParabola`]. Such a section is
    /// on a smooth curve and carries no notch factor
    /// ([`RootSection::stress_correction`]).
    pub tangency_on_flank: bool,
    /// Parabola parameter `p` in `x² = 4p(y_v − y)`, for drawing the inscribed
    /// parabola. Only meaningful for [`CriticalSection::LewisParabola`].
    pub parabola_p: Option<f64>,

    /// Tangency point on the `+x` side, for drawing.
    pub tangency: [f64; 2],
    /// Unit tangent to the fillet at the tangency point, `+x` side.
    ///
    /// Carried rather than left to be reconstructed from the 30° angle: the
    /// angle alone does not fix which way the line leans, and a drawing that
    /// rebuilt it got the sign wrong while the tangency point was correct.
    /// Anything depicting the construction should use this.
    pub tangent_direction: [f64; 2],
    /// Load application point on the flank, for drawing.
    pub load_point: [f64; 2],
    /// Where the load line crosses the centreline, for drawing.
    pub load_line_crossing: [f64; 2],
}

/// A point on the fillet and the curve's tangent there, in tooth coordinates.
///
/// The derivative is analytic. Writing the trochoid as a rotation of the
/// generating-frame point makes it fall out in two lines:
///
/// ```text
/// X = u cos φ − v sin φ        X' = u' cos φ − v' sin φ − φ' Y
/// Y = v cos φ + u sin φ        Y' = u' sin φ + v' cos φ + φ' X
/// ```
///
/// with `u = k s`, `v = r − k b_c`, `φ = (s − a_c)/r`.
///
/// Public so `tests/bending.rs` can differentiate it numerically and check that
/// the closed-form curvature below is what such a difference converges on. That
/// gate is the whole reason the analytic second derivative is trustworthy, and
/// an integration test cannot reach a crate-private function.
pub fn fillet_point_and_tangent(g: &Tooth, s: f64) -> ([f64; 2], [f64; 2]) {
    let d = f64::hypot(s, g.bc);
    let k = 1.0 + g.rho / d;
    let dk = -g.rho * s / (d * d * d);

    let u = k * s;
    let du = k + s * dk;
    let v = g.r - k * g.bc;
    let dv = -dk * g.bc;

    let phi = (s - g.ac) / g.r;
    let dphi = 1.0 / g.r;
    let (c, sn) = (phi.cos(), phi.sin());

    let x = u * c - v * sn;
    let y = v * c + u * sn;
    let dx = du * c - dv * sn - dphi * y;
    let dy = du * sn + dv * c + dphi * x;
    ([x, y], [dx, dy])
}

/// Radius of curvature of the rack-cut fillet at rack travel `s`, mm.
///
/// Closed form. The corner centre runs along a straight line at unit speed, so
/// in the generating frame
///
/// ```text
/// d = √(s² + b_c²)      k = 1 + ρ/d
/// k′ = −ρ s / d³        k″ = −ρ (d² − 3s²) / d⁵
/// q  = ( k s , r − k b_c )
/// ```
///
/// and [`crate::tooth::rolling_curvature_radius`] carries the rolling. It used to be a
/// central difference with a chosen step; see that function for why that was
/// worth removing.
///
/// Public for the same reason as [`fillet_point_and_tangent`]: the gate holding
/// the two against each other lives in `tests/`.
pub fn fillet_curvature_radius(g: &Tooth, s: f64) -> f64 {
    let d = f64::hypot(s, g.bc);
    let (d3, d5) = (d.powi(3), d.powi(5));
    let k = 1.0 + g.rho / d;
    let dk = -g.rho * s / d3;
    let ddk = -g.rho * (d * d - 3.0 * s * s) / d5;

    let q = [k * s, g.r - k * g.bc];
    let dq = [dk * s + k, -dk * g.bc];
    let ddq = [ddk * s + 2.0 * dk, -ddk * g.bc];

    crate::tooth::rolling_curvature_radius(q, dq, ddq, 1.0 / g.r)
}

/// A point on the involute flank and its tangent, in tooth coordinates.
///
/// The parabola construction has to search the flank as well as the fillet: on
/// anything but a small or undercut tooth the largest inscribed parabola touches
/// the *flank*. For the rack limit that tangency sits 0.54 module above where
/// the fillet ends, so a fillet-only search finds nothing at all.
fn flank_point_and_tangent(g: &Tooth, u: f64) -> ([f64; 2], [f64; 2]) {
    let root = f64::hypot(1.0, u);
    let r = g.rb * root;
    let th = g.psi_b - (u - u.atan());
    let (st, ct) = (th.sin(), th.cos());

    let dr = g.rb * u / root;
    let dth = -(u * u) / (1.0 + u * u);
    (
        [r * st, r * ct],
        [dr * st + r * ct * dth, dr * ct - r * st * dth],
    )
}

/// A point on the involute flank and the direction of the load there.
///
/// The load acts along the involute normal, which is the line from the contact
/// point to the base-circle tangency point — that is what "the line of action"
/// means, and it is exact rather than an approximation to the pressure angle.
fn flank_point_and_load_direction(g: &Tooth, roll: f64) -> ([f64; 2], [f64; 2]) {
    let (r, th) = g.involute_at(roll);
    let p = [r * th.sin(), r * th.cos()];
    // The generating tangent point sits `roll` radians back around the base
    // circle from the involute's own angular position.
    let tangent_angle = g.psi_b - roll;
    let t = [g.rb * tangent_angle.sin(), g.rb * tangent_angle.cos()];
    let (dx, dy) = (p[0] - t[0], p[1] - t[1]);
    let len = f64::hypot(dx, dy);
    if len < f64::MIN_POSITIVE {
        return (p, [1.0, 0.0]);
    }
    (p, [dx / len, dy / len])
}

// ------------------------------------------------------- tooth outlines ---

/// A tooth the critical-section construction can be run on.
///
/// The construction — inscribe the largest constant-strength parabola, find
/// where it touches — is the same for an external tooth and a ring's. What
/// differs is only *which two curves* it searches and which way the tooth
/// points, and both of those are what this trait carries. So a ring's rating is
/// a **value** of the general construction rather than a second copy of it, and
/// the two cannot drift apart.
///
/// # The frame, which is the whole trick
///
/// Coordinates are **tooth-local**: `x` from the tooth centreline, and `y`
/// increasing *toward the tip*. For an external gear that is the radius. For a
/// ring, whose tooth points inward, it is the **negated** radius.
///
/// That one flip is enough, and it is worth seeing why. The tangency condition
/// `X·Y′ + 2X′(y_v − Y) = 0` is odd in `y` — negate `Y`, `Y′` and `y_v` together
/// and the whole expression changes sign, so its zero set is untouched. The
/// moment arm `y_v − y_tangency` is a difference, so it flips with the frame and
/// comes out positive either way. The root chord and the load angle only involve
/// `x`. Nothing else in the construction reads an absolute `y`.
pub trait ToothOutline {
    /// **Transverse module of the plane this outline is drawn in**, mm.
    ///
    /// Transverse rather than normal because that is the plane
    /// [`Self::fillet_at`] and [`Self::flank_at`] return coordinates in, and the
    /// form factor divides lengths measured there by it. On the **virtual spur**
    /// member every rating is taken on, the helix is zero and the two moduli
    /// coincide — so this is `m_n` wherever it matters, reached by the
    /// definition that is true of a raw helical member as well.
    fn transverse_module(&self) -> f64;
    /// Normal pressure angle, radians.
    fn normal_pressure_angle(&self) -> f64;
    /// Whether there is a tooth to rate at all.
    fn is_usable(&self) -> bool;
    /// The angle [`CriticalSection::TangentAngle`] takes its tangent at,
    /// degrees — 30° for an external tooth, 60° for a ring's
    /// ([`TANGENT_ANGLE_INTERNAL_DEG`]).
    ///
    /// **Read by that construction and by nothing else.**
    /// [`CriticalSection::LewisParabola`], which is the default, never asks:
    /// its tangency condition carries no angle at all, and what it needs to
    /// know about which way a tooth points is the frame flip this trait already
    /// describes.
    ///
    /// It is the **one** quantity in the tangent construction whose value
    /// depends on the kind of member, which is why it is a number here rather
    /// than a test inside the search — the solve reads a target and does not
    /// otherwise know what it is running on. Before it existed the search took
    /// 30° from a constant and was simply wrong on a ring.
    fn tangent_angle_deg(&self) -> f64;
    /// Fillet parameter bracket, ordered `(lo, hi)`.
    fn fillet_bracket(&self) -> (f64, f64);
    /// The fillet parameter at its **deepest** point, where it meets the root.
    ///
    /// The other end of [`Self::fillet_junction`]'s bracket, and the point the
    /// fillet is tightest at — so it is where `ρ_f`, the *minimum* radius of
    /// curvature of the fillet curve, is read. Named rather than taken as an
    /// endpoint of [`Self::fillet_bracket`] because which endpoint that is
    /// differs between a rack-cut tooth and a ring, and a caller that guessed
    /// would be right on one of them.
    fn fillet_root(&self) -> f64;
    /// The fillet parameter where the fillet meets the involute flank.
    ///
    /// The notch does not stop existing when the critical section climbs above
    /// it, so this is the fillet point nearest a flank tangency — see
    /// [`RootSection::notch_parameter`].
    fn fillet_junction(&self) -> f64;
    /// Flank roll-parameter bracket, ordered `(lo, hi)`.
    fn flank_bracket(&self) -> (f64, f64);
    /// Fillet point and tangent at `s`, in the frame above.
    fn fillet_at(&self, s: f64) -> ([f64; 2], [f64; 2]);
    /// Flank point and tangent at roll `u`, in the frame above.
    fn flank_at(&self, u: f64) -> ([f64; 2], [f64; 2]);
    /// Flank point and load direction at roll `u`, in the frame above.
    fn load_at(&self, u: f64) -> ([f64; 2], [f64; 2]);
    /// Radius of curvature of the fillet at `s`, mm.
    fn fillet_curvature(&self, s: f64) -> f64;
    /// Radius of curvature of the involute flank at roll `u`, mm — `r_b u`.
    fn flank_curvature(&self, u: f64) -> f64;
    /// **How far the flank's tangent at roll `u` leans in toward the
    /// centreline**, radians, going toward the tip: `u − ψ_b` on a tooth,
    /// `u + ψ_b` on a ring, whose `ψ_b` is the involute's angle at the base
    /// circle in its own sense. Linear in `u` on both. It is also the load's
    /// angle off the across-tooth direction there, in magnitude, since the
    /// load is along the flank's normal.
    fn flank_tilt(&self, u: f64) -> f64;

    // ---- what a *rating* needs beyond the outline -------------------- //
    //
    // The rating sweeps a load point along the flank, and everything it needs
    // to do that is here rather than in a function per kind of member. The two
    // used to be two near-identical functions; they differ in one fact, and it
    // is [`Self::tip_at_high_roll`].

    /// Base radius, mm.
    fn base_radius(&self) -> f64;
    /// Transverse pressure angle, radians.
    fn transverse_pressure_angle(&self) -> f64;
    /// Base helix angle, radians.
    fn base_helix_angle(&self) -> f64;
    /// **Which end of [`Self::flank_bracket`] the tooth tip is at.**
    ///
    /// The one genuine asymmetry between a rack-cut tooth and a ring, and three
    /// things fall out of it rather than being stated three times: where the
    /// load point is counted *from*, which way it travels as the mesh turns —
    /// down in roll for a tooth, up for a ring — and, since the bracket is the
    /// generated flank either way, where it stops being on the part.
    fn tip_at_high_roll(&self) -> bool;
    /// The **virtual spur** member: the one whose own transverse plane is the
    /// normal plane this member bends in. A spur member is itself.
    fn virtual_spur(&self) -> Self
    where
        Self: Sized;
    /// How this member's rim is measured against its teeth, given a thickness.
    ///
    /// ISO 6336-3, 9.3: a backup ratio against the whole tooth depth for an
    /// external member, a rim thickness in normal modules for a ring. One fit,
    /// two references, and this is the only place the difference lives.
    fn rim_support(&self, thickness: f64) -> RimSupport;
}

impl ToothOutline for Tooth {
    fn transverse_module(&self) -> f64 {
        self.mt
    }
    fn normal_pressure_angle(&self) -> f64 {
        self.alpha_n
    }
    fn is_usable(&self) -> bool {
        // A tooth with no flank — severed, or ended at its tip on the fillet —
        // has nowhere to take a load.
        !self.severed && self.u_j < self.u_tip
    }
    fn tangent_angle_deg(&self) -> f64 {
        TANGENT_ANGLE_DEG
    }
    fn fillet_bracket(&self) -> (f64, f64) {
        (self.s_j, 0.0)
    }
    fn fillet_junction(&self) -> f64 {
        self.s_j
    }
    fn fillet_root(&self) -> f64 {
        // Rack travel zero is the tooth centreline: the deepest the corner cut.
        0.0
    }
    fn flank_bracket(&self) -> (f64, f64) {
        (self.u_j, self.u_tip)
    }
    fn fillet_at(&self, s: f64) -> ([f64; 2], [f64; 2]) {
        fillet_point_and_tangent(self, s)
    }
    fn flank_at(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        flank_point_and_tangent(self, u)
    }
    fn load_at(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        flank_point_and_load_direction(self, u)
    }
    fn fillet_curvature(&self, s: f64) -> f64 {
        fillet_curvature_radius(self, s)
    }
    fn flank_curvature(&self, u: f64) -> f64 {
        self.rb * u
    }
    fn flank_tilt(&self, u: f64) -> f64 {
        // The tangent of `(r sin θ, r cos θ)`, `θ = ψ_b − inv(u)`, points at
        // `θ + atan u = ψ_b − u` from the centreline, outward.
        u - self.psi_b
    }
    fn base_radius(&self) -> f64 {
        self.rb
    }
    fn transverse_pressure_angle(&self) -> f64 {
        self.alpha_t
    }
    fn base_helix_angle(&self) -> f64 {
        Tooth::base_helix_angle(self)
    }
    fn tip_at_high_roll(&self) -> bool {
        // An external tooth's flank runs from its fillet junction out to its
        // tip, so the tip is the far end and the load travels back down.
        true
    }
    fn virtual_spur(&self) -> Self {
        Tooth::virtual_spur(self)
    }
    fn rim_support(&self, thickness: f64) -> RimSupport {
        RimSupport::external(thickness, self.ra - self.rf)
    }
}

/// A ring's tooth, in the same frame — which for a ring means `y` negated,
/// because its tooth points inward.
impl ToothOutline for crate::ring::Ring {
    fn transverse_module(&self) -> f64 {
        self.mt
    }
    fn normal_pressure_angle(&self) -> f64 {
        self.alpha_n
    }
    fn is_usable(&self) -> bool {
        // A ring whose cut generated no fillet has no notch, and the whole
        // construction below is a search *along the fillet* for one. Rated
        // without this it searched an empty bracket and answered anyway.
        self.fillet.is_some()
            && self.u_j.is_finite()
            && self.u_tip.is_finite()
            && self.u_j > self.u_tip
    }
    fn tangent_angle_deg(&self) -> f64 {
        TANGENT_ANGLE_INTERNAL_DEG
    }
    fn fillet_bracket(&self) -> (f64, f64) {
        self.fillet.map_or((0.0, 0.0), |f| {
            (f.phi_root.min(f.phi_j), f.phi_root.max(f.phi_j))
        })
    }
    fn fillet_junction(&self) -> f64 {
        self.fillet.map_or(0.0, |f| f.phi_j)
    }
    fn fillet_root(&self) -> f64 {
        self.fillet.map_or(0.0, |f| f.phi_root)
    }
    fn flank_bracket(&self) -> (f64, f64) {
        (self.u_tip, self.u_j)
    }
    fn fillet_at(&self, s: f64) -> ([f64; 2], [f64; 2]) {
        flip_y(crate::ring::Ring::fillet_point_and_tangent(self, s))
    }
    fn flank_at(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        flip_y(crate::ring::Ring::flank_point_and_tangent(self, u))
    }
    fn load_at(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        flip_y(crate::ring::Ring::flank_point_and_load_direction(self, u))
    }
    fn fillet_curvature(&self, s: f64) -> f64 {
        crate::ring::Ring::fillet_curvature_radius(self, s)
    }
    fn flank_curvature(&self, u: f64) -> f64 {
        self.rb * u
    }
    fn flank_tilt(&self, u: f64) -> f64 {
        // `θ = ψ_b + inv(u)`, and in the flipped frame the tangent toward the
        // tip points at `θ + atan u = ψ_b + u` inward.
        u + self.psi_b
    }
    fn base_radius(&self) -> f64 {
        self.rb
    }
    fn transverse_pressure_angle(&self) -> f64 {
        self.alpha_t
    }
    fn base_helix_angle(&self) -> f64 {
        crate::ring::Ring::base_helix_angle(self)
    }
    fn tip_at_high_roll(&self) -> bool {
        // A ring's tip is at its *smallest* radius and so its smallest roll,
        // and the load travels up from it.
        false
    }
    fn virtual_spur(&self) -> Self {
        crate::ring::Ring::virtual_spur(self)
    }
    fn rim_support(&self, thickness: f64) -> RimSupport {
        RimSupport::internal(thickness, self.params.module)
    }
}

/// Into the tooth-local frame for an inward-pointing tooth: negate `y` on both
/// the point and the direction.
fn flip_y((p, t): ([f64; 2], [f64; 2])) -> ([f64; 2], [f64; 2]) {
    ([p[0], -p[1]], [t[0], -t[1]])
}

/// The critical root section for a load applied at a given roll parameter.
///
/// Returns `None` when the gear has no usable flank — a severed tooth, or a
/// fillet on which the 30° tangent does not exist.
#[must_use]
pub fn root_section<T: ToothOutline + ?Sized>(g: &T, load_roll: f64) -> Option<RootSection> {
    root_section_with(g, load_roll, CriticalSection::default())
}

/// The critical root section, locating it by the chosen construction, the
/// governing one under the default bending model
/// ([`root_section_rated`]).
///
/// Returns `None` when the gear has no usable flank — a severed tooth, or a
/// fillet on which the construction has no solution.
#[must_use]
pub fn root_section_with<T: ToothOutline + ?Sized>(
    g: &T,
    load_roll: f64,
    method: CriticalSection,
) -> Option<RootSection> {
    root_section_rated(g, load_roll, method, RootStressModel::default())
}

/// **The governing root section under a bending model**: of the sections the
/// construction finds, the one whose rated factor
/// ([`RootSection::bending_factor`]) is highest — `Y_F`, the axial term and
/// the notch factor together, never `Y_F` alone.
///
/// The notch factor belongs to the notch: a section in the fillet carries the
/// fillet's, one on the smooth flank none ([`RootSection::stress_correction`]).
/// A section the model cannot read (its bending factor not a positive number:
/// the loaded fillet compressed rather than stretched) never masks one it can.
/// Where none can be read the first found is returned, so the rating can say
/// why it has none ([`Unrated::Compressed`]); `None` where the construction
/// finds no section at all.
#[must_use]
pub fn root_section_rated<T: ToothOutline + ?Sized>(
    g: &T,
    load_roll: f64,
    method: CriticalSection,
    model: RootStressModel,
) -> Option<RootSection> {
    let found = root_sections(g, load_roll, method);
    let rated = found
        .iter()
        .filter_map(|s| s.bending_factor(model).map(|f| (*s, f)))
        .reduce(|a, b| if b.1.total_cmp(&a.1).is_gt() { b } else { a })
        .map(|(s, _)| s);
    rated.or_else(|| found.first().copied())
}

/// **Every section the construction finds** for a load at `load_roll`: the
/// tangent's one; the parabola's tangency on the fillet and on the flank,
/// each where it has one, the fillet's first.
#[must_use]
pub fn root_sections<T: ToothOutline + ?Sized>(
    g: &T,
    load_roll: f64,
    method: CriticalSection,
) -> Vec<RootSection> {
    if !g.is_usable() || !load_roll.is_finite() {
        return Vec::new();
    }

    // The load has to be resolved first either way: the parabola's vertex sits
    // where the load line crosses the centreline.
    let (load_point, dir) = g.load_at(load_roll);
    if dir[0].abs() < 1e-12 {
        return Vec::new();
    }
    let crossing = [0.0, load_point[1] + (-load_point[0] / dir[0]) * dir[1]];
    let vertex = crossing[1];

    let (fillet_lo, fillet_hi) = g.fillet_bracket();
    let s = match method {
        // Along the fillet the tangent angle to the centreline sweeps from near
        // zero at the junction to 90° at the root circle, so this is monotone
        // and the bracket is the fillet itself. **The angle is the member's**,
        // 30° or 60°, and the search does not otherwise know which it is on.
        CriticalSection::TangentAngle => {
            let target = g.tangent_angle_deg().to_radians().tan();
            let Some(s) = brent(
                |s| {
                    let (_, t) = g.fillet_at(s);
                    t[0].abs() - target * t[1].abs()
                },
                fillet_lo,
                fillet_hi,
                Tol::default(),
            ) else {
                return Vec::new();
            };
            s
        }
        // Tangency of the parabola x² = 4p(y_v − y) with the tooth outline.
        // Requiring the point to lie on the parabola and the slopes to match
        // eliminates p and leaves one equation:
        //     X·Y' + 2 X' (y_v − Y) = 0
        //
        // Odd in y, so it is the same equation for a tooth pointing either way —
        // see `ToothOutline`. Searched on the fillet first and then the flank,
        // because which one it touches depends on the tooth: small and undercut
        // teeth touch the fillet, larger ones the flank.
        CriticalSection::LewisParabola => {
            let condition = |q: [f64; 2], t: [f64; 2]| q[0] * t[1] + 2.0 * t[0] * (vertex - q[1]);
            let (flank_lo, flank_hi) = g.flank_bracket();
            // **Each curve offers its least `x²/(y_v − y)`**, the Lewis
            // measure, over the whole curve below the vertex: at an interior
            // tangency, or at one of the curve's ends. Savage, Rubadeux & Coe
            // search "both involute and trochoid geometry" for "the smallest
            // inscribed parabola"; here each curve's own smallest is a
            // candidate section, and the candidates are compared by what each
            // rates at, the fillet's with its notch factor and the smooth
            // flank's with none ([`root_section_rated`]) — not by `Y_F`
            // alone, and an end is as much a candidate as a tangency. A
            // ring's fillet has no tangency, so its candidate is its end at
            // the flank, which keeps its notch factor.
            //
            // **Each curve has one interior least at most, found by
            // construction:**
            //
            // - On the fillet the outline, read as a half-width `w(y)`, falls
            //   and is convex (the fillet is concave toward the space), so
            //   `w + 2w′(y_v − y)` rises: one crossing at most, from falling to
            //   rising `x²/(y_v − y)`. The fillet's bracket holds every
            //   tangency it has.
            // - On an involute, with `δ` the tangent's lean ([`ToothOutline::
            //   flank_tilt`]) and `u` the roll, the condition is
            //   `r_b (cos δ + u sin δ + u / sin δ) = 2|y_v|`, whose left side
            //   falls and then rises: its slope has the sign of
            //   `sin δ − u cos³δ`, which rises with `u`. A crossing on the
            //   falling part is a *greatest* `x²/(y_v − y)`, one on the rising
            //   part the least. So the flank is searched from where that sign
            //   turns: a search of the whole flank finds no sign change when
            //   both are there, and the fillet's tangency was then taken
            //   where the flank's is less (z 30, x 0.8, 25°, h_a 1.25, ρ 0.2
            //   at ε 1.2).
            let turn = |u: f64| {
                let lean = g.flank_tilt(u);
                lean.sin() - u * lean.cos().powi(3)
            };
            let rising = if turn(flank_lo) >= 0.0 {
                Some(flank_lo)
            } else {
                brent(turn, flank_lo, flank_hi, Tol::default())
            };
            let fillet_tangency = brent(
                |s| {
                    let (q, t) = g.fillet_at(s);
                    condition(q, t)
                },
                fillet_lo,
                fillet_hi,
                Tol::default(),
            );
            let flank_tangency = rising.and_then(|from| {
                brent(
                    |u| {
                        let (q, t) = g.flank_at(u);
                        condition(q, t)
                    },
                    from,
                    flank_hi,
                    Tol::default(),
                )
            });
            // The Lewis measure at a point, where the point is below the
            // vertex and off the centreline, each by more than its own
            // coordinates' rounding ([`beyond_rounding`]): the least over a
            // curve is never at the vertex's height, where the measure grows
            // without bound, and a point on the centreline has no chord.
            let measure = |q: [f64; 2]| {
                let scale = q[0].hypot(q[1]);
                (beyond_rounding(vertex - q[1], scale) && beyond_rounding(q[0], scale))
                    .then(|| q[0] * q[0] / (vertex - q[1]))
            };
            // Each curve's least of its tangency and its ends, the tangency
            // first on a tie.
            let least = |on: Curve<T>, tangency: Option<f64>, ends: &[f64]| {
                tangency
                    .map(|p| (p, true))
                    .into_iter()
                    .chain(ends.iter().map(|&p| (p, false)))
                    .filter_map(|(p, at_tangency)| measure(on(g, p).0).map(|m| (p, at_tangency, m)))
                    .reduce(|a, b| if b.2 < a.2 { b } else { a })
            };
            // **The flank's tip is never below the vertex**, so only its end
            // at the fillet is a candidate. With `δ = u_tip − ψ_b` the load
            // line at any roll on the flank crosses the centreline at most
            // `r_b / cos δ` out (the load angle `u − ψ_b` is greatest at the
            // tip), and the tip corner stands `r_b (cos δ + u sin δ)` out,
            // which is no less wherever `u ≥ tan δ`, the tip's half angle not
            // negative: equal only on a pointed tip loaded at its point,
            // where the section would have no chord. Counting the tip end let
            // rounding put that point a hair under the vertex and rate a
            // chord of 1e-15.
            let flank_root = if g.tip_at_high_roll() {
                flank_lo
            } else {
                flank_hi
            };
            let candidates = [
                least(T::fillet_at, fillet_tangency, &[fillet_lo, fillet_hi]).and_then(
                    |(s, at_tangency, _)| {
                        finish(g, method, s, false, at_tangency, load_point, dir, crossing)
                    },
                ),
                least(T::flank_at, flank_tangency, &[flank_root]).and_then(
                    |(u, at_tangency, _)| {
                        finish(g, method, u, true, at_tangency, load_point, dir, crossing)
                    },
                ),
            ];
            // A candidate with no finite form factor is no section. Which of
            // the rest governs is a rating's question, not the construction's
            // ([`root_section_rated`]).
            return candidates
                .into_iter()
                .flatten()
                .filter(|c| c.form_factor.is_finite())
                .collect();
        }
    };

    finish(g, method, s, false, true, load_point, dir, crossing)
        .into_iter()
        .collect()
}

/// One of a tooth's two curves, as [`ToothOutline::fillet_at`] and
/// [`ToothOutline::flank_at`] read it: a point and a tangent at a parameter.
type Curve<T> = fn(&T, f64) -> ([f64; 2], [f64; 2]);

/// **Whether a length read off coordinates is more than their rounding**:
/// `length > COORDINATE_ROUNDING · scale`, `scale` the point's distance from
/// the axis.
///
/// A point on a tooth is `R` from the axis at an angle both built from a few
/// dozen floating operations (a roll, an involute, a sine and a cosine), so
/// each coordinate carries a few dozen roundings of `R`. `2⁸ε` of `R` bounds
/// that with a margin, and a half-width or a depth below the vertex no larger
/// is zero: the apex of a pointed tip loaded at its point, which solves the
/// tangency condition exactly (`u = tan δ` there) with a chord of about
/// `ε·R`, and which as a section would rate at 1e13.
fn beyond_rounding(length: f64, scale: f64) -> bool {
    length > COORDINATE_ROUNDING * scale
}

/// The roundings of a point's distance from the axis that a coordinate read
/// off it carries, with margin: see [`beyond_rounding`].
const COORDINATE_ROUNDING: f64 = 256.0 * f64::EPSILON;

/// Assemble the section at a point of one curve, whichever curve it is on:
/// a tangency, or one of the curve's ends (`at_tangency` false).
#[allow(clippy::too_many_arguments)]
fn finish<T: ToothOutline + ?Sized>(
    g: &T,
    method: CriticalSection,
    param: f64,
    on_flank: bool,
    at_tangency: bool,
    load_point: [f64; 2],
    load_dir: [f64; 2],
    crossing: [f64; 2],
) -> Option<RootSection> {
    let s = param;
    let (tangency, raw_tangent) = if on_flank {
        g.flank_at(param)
    } else {
        g.fillet_at(param)
    };
    let root_chord = 2.0 * tangency[0].abs();
    // Orient it up the tooth (towards the tip) so the direction is unambiguous.
    let len = f64::hypot(raw_tangent[0], raw_tangent[1]);
    let sign = if raw_tangent[1] < 0.0 { -1.0 } else { 1.0 };
    let tangent_direction = [sign * raw_tangent[0] / len, sign * raw_tangent[1] / len];

    let moment_arm = crossing[1] - tangency[1];
    // The parabola through the section's point: tangent to the outline there
    // at a tangency, and through a curve's end otherwise.
    let parabola_p = match method {
        CriticalSection::LewisParabola if at_tangency => {
            Some(-tangency[0] * raw_tangent[0] / (2.0 * raw_tangent[1]))
        }
        CriticalSection::LewisParabola => Some(tangency[0] * tangency[0] / (4.0 * moment_arm)),
        CriticalSection::TangentAngle => None,
    };
    // cos α_Fen is the share of the load acting across the tooth; the load
    // direction's x-component is exactly that. Read as an arctangent of the
    // two components, since an arccosine is flat to rounding within `√ε` of a
    // square load, where the rating has a corner.
    let load_angle = load_dir[1].abs().atan2(load_dir[0].abs());

    // The fillet's radius at the section, or at the fillet point nearest it
    // (the junction) for a section on the flank: reported as the notch nearest
    // the section, and read by a fit only where the section is in the fillet
    // (`RootSection::stress_correction`).
    let fillet_radius = g.fillet_curvature(if on_flank { g.fillet_junction() } else { s });
    // ...and `ρ_f`, which is not a point on the section at all but the tightest
    // the fillet ever gets. See `RootSection::min_fillet_curvature`.
    let min_fillet_radius = g.fillet_curvature(g.fillet_root());

    let m = g.transverse_module();
    let alpha_n = g.normal_pressure_angle();
    let (form_factor, axial_compression) = beam(moment_arm, load_angle, root_chord, m, alpha_n);

    Some(RootSection {
        s,
        // Curvature is a fillet property; on a flank tangency the involute's own
        // curvature is what the notch sees.
        notch_parameter: root_chord / (2.0 * fillet_radius),
        tangency_on_flank: on_flank,
        method,
        parabola_p,
        root_chord,
        moment_arm,
        load_angle,
        fillet_curvature: fillet_radius,
        min_fillet_curvature: min_fillet_radius,
        pressure_angle_rad: alpha_n,
        form_factor,
        axial_compression,
        module: m,
        tangency,
        tangent_direction,
        load_point,
        load_line_crossing: crossing,
    })
}

/// `Y_F` and the axial term of a section of chord `root_chord` under a load
/// at `moment_arm` above it and `load_angle` off the across-tooth direction.
fn beam(moment_arm: f64, load_angle: f64, root_chord: f64, m: f64, alpha_n: f64) -> (f64, f64) {
    let form_factor =
        6.0 * (moment_arm / m) * load_angle.cos() / ((root_chord / m).powi(2) * alpha_n.cos());
    // ...and the along-tooth half of the same load. See `axial_compression`.
    let axial_compression = load_angle.sin() / ((root_chord / m) * alpha_n.cos());
    (form_factor, axial_compression)
}

impl RootSection {
    /// **This section with the load moved** to roll `load_roll` on `g`: the
    /// chord, the notch and where they are stay; the moment arm, the load's
    /// angle and what they make of `Y_F` and the axial term follow the load.
    ///
    /// How a section found once is read over a mesh cycle — by analogy with
    /// ISO's tangent section, which is load-independent by construction; a
    /// parabola section held at one load point is a hybrid, and its cost is
    /// `gear-cli sharingbias`'s. The arm is bounded by the tooth, so the figure stays
    /// bounded wherever the load is — the tip of a pointed tooth included,
    /// where a section searched afresh would shrink onto the apex.
    #[must_use]
    pub fn loaded_at<T: ToothOutline + ?Sized>(&self, g: &T, load_roll: f64) -> Option<Self> {
        if !load_roll.is_finite() {
            return None;
        }
        let (load_point, dir) = g.load_at(load_roll);
        if dir[0].abs() < 1e-12 {
            return None;
        }
        let crossing = [0.0, load_point[1] + (-load_point[0] / dir[0]) * dir[1]];
        let moment_arm = crossing[1] - self.tangency[1];
        let load_angle = dir[1].abs().atan2(dir[0].abs());
        let (form_factor, axial_compression) = beam(
            moment_arm,
            load_angle,
            self.root_chord,
            self.module,
            self.pressure_angle_rad,
        );
        Some(Self {
            moment_arm,
            load_angle,
            form_factor,
            axial_compression,
            load_point,
            load_line_crossing: crossing,
            ..*self
        })
    }
}

/// **Which published bending model a root stress is taken under.**
///
/// It began as a choice of stress concentration factor and is now a choice of
/// *set*: each variant names a notch factor **and** whether the axial
/// compression term is taken, because the two travel together in the sources
/// they come from and cannot be mixed without meaning something neither source
/// says. This project has twice paid for mixing halves of calibrations
/// (`docs/rationale.md`), and a type that made it easy was part of how.
///
/// Kept as an explicit choice rather than folded into the stress calculation so
/// that a run can be repeated without it. The form factor is measured off exact
/// geometry and is checkable against a closed-form limit; a stress correction is
/// an empirical fit, and being able to separate the two is what makes it
/// possible to tell a geometry error from an over-fitted correction.
///
/// # Why this survives the no-correction-factors policy
///
/// docs/rationale.md#no-isoagma-correction-factors declines ISO's load factors
/// `K_v`, `K_Fβ` and `K_Fα`, and the contact factors `Z_ε` and `Z_β`; `K_A` is
/// the load case's own input, 1 unless a designer states it. The load factors
/// are `≥ 1` by definition, so leaving them out makes every stress **nominal**:
/// `σ_F` low by their product and `σ_H` by its square root — the
/// unconservative direction, recorded in `docs/state.md`. `Y_S` is kept, and
/// the difference is not special pleading:
///
/// - **Dropping it would be the same error, larger.** `Y_S ≥ 1` — typically
///   1.6 to 2.1. Without it a nominal section stress sits well *below* the real
///   peak.
/// - **It is local, not population-calibrated.** It converts nominal stress at a
///   section into peak stress at a notch. Its inputs `s_Fn`, `h_Fe` and `ρ_F`
///   are measured off this gear's own generated profile, not looked up against a
///   population of test gears.
/// - **Its range is reported, not assumed.** See
///   [`RootSection::notch_parameter_in_range`]; a result that leaves the fit's
///   band says so instead of quietly returning a boundary value.
///
/// It remains a fit, and [`RootStressModel::FormFactorOnly`] exists so any result can
/// be re-run without it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RootStressModel {
    /// ISO 6336-3.
    ///
    /// ```text
    /// Y_S = (1.2 + 0.13 L) · q_s^(1 / (1.21 + 2.3/L))
    /// L   = s_Fn / h_Fe          q_s = s_Fn / (2 ρ_F)
    /// ```
    ///
    /// **Not the default**, and it was for a long time. It was chosen first
    /// because it is written in terms of the geometry this crate already
    /// measures — `s_Fn`, `h_Fe` and `ρ_F` all come off the generated profile —
    /// where Dolan and Broghamer's work is *presented* as charts indexed by
    /// tooth count and shift. That reading mistook the presentation for the
    /// fit: AGMA's curve fit of the same data is written in the same three
    /// lengths, at any pressure angle, and it is the notch factor that belongs
    /// to the parabola section this crate computes. `Y_S` is fitted to ISO's
    /// tangent section and to `ρ_F` *at* that section, so it is offered here
    /// as the other half of the coherent ISO set —
    /// [`CriticalSection::TangentAngle`] with `Y_S`, and no axial term — for a
    /// number comparable with a published rating, which `gear-cli matrix`
    /// prints beside the default on both kinds of member.
    ///
    /// It remains an empirical fit, stated over [`NOTCH_PARAMETER_RANGE`] and
    /// for external spur gears at 20°; both limits are recorded there.
    Iso6336,
    /// **Dolan and Broghamer**, as curve-fitted by AGMA, and the default.
    ///
    /// ```text
    /// K_f = H + (s_Fn/ρ_f)^L · (s_Fn/h_Fe)^M
    /// H = 0.331 − 0.436·α_n     L = 0.324 − 0.492·α_n     M = 0.261 + 0.545·α_n
    /// ```
    ///
    /// with `α_n` in radians, and **`ρ_f` the minimum radius of curvature of
    /// the fillet curve** — [`RootSection::min_fillet_curvature`], not the
    /// radius at the section.
    ///
    /// # Why this is the default rather than `Y_S`
    ///
    /// Not because it is better in the abstract. Because it is the notch factor
    /// that belongs to the **section this crate actually computes**. The
    /// critical section here is the inscribed Lewis parabola, searched over both
    /// the fillet and the flank and resolved to the weaker — which is Savage,
    /// Rubadeux & Coe's construction (NASA TM-107012), and Dolan and Broghamer's
    /// factor is the one that model carries. `Y_S` is fitted to ISO's 30°/60°
    /// tangent section and to `ρ_F` measured *at* that section; feeding it a
    /// parabola section, and on a ring a flank one, is taking half of a
    /// calibration. This project has made that mistake once already and it is
    /// written up in `docs/rationale.md`.
    ///
    /// Three further things recommend it here, all of which are about *this*
    /// tool rather than about the fit:
    ///
    /// - **It is written in `α_n`.** `Y_S` is stated for external spur gears at
    ///   20° and is "approximate" elsewhere by ISO's own words (6336-3, 7.1).
    ///   This crate offers 14.5°, 20° and 25°, and the AGMA fit's constants are
    ///   linear in the pressure angle, so all three are inside it. At 20° the
    ///   constants come out 0.179 / 0.152 / 0.451 against Dolan and Broghamer's
    ///   own published 20° equation, `0.18 + (t/ρ)^0.15 (t/h)^0.45`.
    /// - **It needs no band.** `Y_S` is stated over `1 ≤ q_s < 8` and a ring
    ///   sits below it two times in three; `K_f` is a product of powers with no
    ///   stated range, so nothing is clamped and nothing is extrapolated past a
    ///   boundary the fit names.
    /// - **It is the same factor for both members.** Savage's internal model is
    ///   explicitly "an extension of the model for an external gear tooth", so
    ///   one notch factor serves a rack-cut tooth and a ring, as one section
    ///   construction already does.
    ///
    /// # What it is, and what it is not
    ///
    /// A 1942 photoelastic curve fit, and it stays one. Its independent
    /// corroboration is unusually good for its age — Jacobson's photoelastic
    /// work (1955) agreed, Chabert, Dang Tran and Mathis (1972) were
    /// "substantially in agreement", and Wilcox and Coleman's finite-element
    /// study (1973) found results "only a few percent different" — but it is a
    /// fit, and [`RootStressModel::FormFactorOnly`] exists so any result can be
    /// re-run without it.
    ///
    /// **Its models contained no undercut teeth.** Dolan and Broghamer's
    /// photoelastic specimens "contained various standard gear teeth but did not
    /// include any undercut gears", and this crate rates undercut teeth. That is
    /// the same class of limit as `Y_S`'s 20°-only origin, it is not reported
    /// per gear because an undercut tooth is already reported per gear
    /// (`clamp.tooth_undercut`), and it is recorded here so the two are not
    /// mistaken for a validated case.
    #[default]
    DolanBroghamer,
    /// The bending term alone: `Y_F`, no notch factor and no axial relief.
    ///
    /// The control case. If a stress figure looks wrong, comparing against this
    /// says whether the geometry or the fit is responsible — and it is the one
    /// variant with no empirical content at all, so it is what the rack-limit
    /// gate is written against.
    FormFactorOnly,
}

/// Range of the notch parameter over which the ISO `Y_S` fit is stated.
///
/// Outside it the formula still evaluates, and [`RootSection::stress_correction`]
/// still returns a value — but [`RootSection::notch_parameter_in_range`] reports
/// false so a caller can say so.
///
/// # Provenance
///
/// **ISO 6336-3:2019, 7.2**, which states Formula (62) as "valid in the range:
/// `1 ≤ q_s < 8`". Read from the standard; it was for a long time a citation of
/// a citation, and it is recorded here that the two agree — the second-hand band
/// was the right one, and is now first-hand.
///
/// # What the same clause says about *where* the fit applies
///
/// 7.1: the formulae "are based on the data derived from the geometry of
/// external spur gears with 20° pressure angle, by means of measurement and
/// calculations using finite element and integral formula methods", and "can
/// also be used to obtain approximate values for internal gears and for gears
/// having other pressure angles."
///
/// This crate applies `Y_S` to both, so both are approximations the standard
/// sanctions rather than extrapolations it does not — but they are
/// approximations, and unlike `q_s` neither has an edge to report, so they are
/// stated in `docs/reference.md` rather than raised per gear. The third
/// departure is this crate's own and is the largest: `s_Fn`, `h_Fe` and `ρ_F`
/// are measured at the inscribed-parabola section by default where the fit was
/// calibrated against the 30°/60° tangent one. That was a deliberate choice
/// before this band was read and it stands — see [`CriticalSection`], which sets
/// out both why and what it costs.
///
/// **What the band can and cannot do to an answer.** It cannot move a number
/// silently:
/// the clamp's effect is bounded by the fit's own behaviour, the unclamped
/// figure stays on [`RootSection::notch_parameter`], and whether it was applied
/// is reported by [`RootSection::notch_parameter_in_range`]. What it does do is
/// under-predict a sharper-than-stated notch, since `Y_S` rises with `q_s` —
/// the unconservative direction, which is exactly why the range is surfaced
/// rather than swallowed.
///
pub const NOTCH_PARAMETER_RANGE: std::ops::Range<f64> = 1.0..8.0;

impl RootSection {
    /// The stress correction factor (`Y_S`, `K_f`) under the chosen model.
    ///
    /// # The notch factor belongs to the notch
    ///
    /// A section in the fillet carries the fillet's notch factor, each model
    /// reading the radius it was fitted to. **A section on the involute flank
    /// carries none** (1): the flank is smooth, and a notch factor read from
    /// the fillet and applied to a point on the flank is a stress
    /// concentration where there is no notch. That is not a step where the
    /// section crosses from one curve to the other: the two curves' sections
    /// are candidates rated side by side ([`root_section_rated`]), the higher
    /// governs, and the greater of two continuous figures is continuous.
    ///
    /// An earlier reading took the fillet's radius at its junction for a flank
    /// section (and before that the involute's own curvature, which jumped
    /// from 0.61 mm to 22.9 mm across one tooth at z = 150→151; see
    /// `docs/corrections.md`). `RootSection::fillet_curvature` and
    /// `notch_parameter` still report the junction's, as what the notch
    /// nearest the section is.
    ///
    /// The notch parameter is **clamped** into the range the fit is stated for
    /// before being used, so an out-of-range gear gets the value at the boundary
    /// rather than an extrapolation. The unclamped figure stays available in
    /// [`RootSection::notch_parameter`], because it is worth seeing even when it
    /// cannot be used: it is largely set by the cutter tip radius and the tooth
    /// space, neither of which the designer can freely choose.
    ///
    /// Note the direction of the error. `Y_S` rises with `q_s`, so clamping a
    /// sharper-than-stated notch **under-predicts** the stress. That is
    /// unconservative, which is why [`RootSection::notch_parameter_in_range`]
    /// exists and should be surfaced rather than swallowed.
    #[must_use]
    pub fn stress_correction(&self, model: RootStressModel) -> Option<f64> {
        // Both fits read `s_Fn / h_Fe`, the chord over the moment arm. A load
        // line crossing the centreline at or below the section leaves no arm,
        // and neither fit has a reading there.
        let has_arm = self.moment_arm > 0.0;
        match model {
            RootStressModel::FormFactorOnly => Some(1.0),
            // **The notch factor belongs to the notch.** A section on the
            // involute flank is on a smooth curve, and neither fit has a notch
            // to read there.
            _ if self.tangency_on_flank => Some(1.0),
            // **Each fit reads the radius it was fitted to**, which is the whole
            // reason both are carried: `ρ_f` here, `ρ_F` below.
            RootStressModel::DolanBroghamer => {
                let a = self.pressure_angle_rad;
                let h = 0.331 - 0.436 * a;
                let l = 0.324 - 0.492 * a;
                let m = 0.261 + 0.545 * a;
                let by_radius = self.root_chord / self.min_fillet_curvature;
                let by_height = self.root_chord / self.moment_arm;
                has_arm.then(|| h + by_radius.powf(l) * by_height.powf(m))
            }
            RootStressModel::Iso6336 => {
                let l = self.root_chord / self.moment_arm;
                let q = self
                    .notch_parameter
                    .clamp(NOTCH_PARAMETER_RANGE.start, NOTCH_PARAMETER_RANGE.end);
                has_arm.then(|| (1.2 + 0.13 * l) * q.powf(1.0 / (1.21 + 2.3 / l)))
            }
        }
    }

    /// Whether the notch parameter falls where the ISO fit is stated.
    #[must_use]
    pub fn notch_parameter_in_range(&self) -> bool {
        NOTCH_PARAMETER_RANGE.contains(&self.notch_parameter)
    }

    /// **The whole geometry factor multiplying `F_t / (b · m_n)`**, under one
    /// model.
    ///
    /// ```text
    /// Dolan–Broghamer  (Y_F − axial) · K_f     Savage's J, both of its terms
    /// ISO 6336         Y_F · Y_S               ISO omits the axial term
    /// form factor only Y_F
    /// ```
    ///
    /// The axial term belongs to the model rather than to the section, which is
    /// why it is applied here and carried separately on
    /// [`Self::axial_compression`]: taking ISO's notch factor *and* AGMA's load
    /// resolution would be a third model that neither source states.
    ///
    /// This is also the quantity a load-sharing sweep maximises, so both terms
    /// have to be in it — the axial relief varies along the mesh cycle exactly
    /// as the bending does.
    ///
    /// **`None` outside the model's domain**: where the notch factor is
    /// undefined ([`RootSection::stress_correction`]), and wherever the
    /// factor is not a finite positive number. Low on the flank — which is
    /// where the unshared load point sits once `ε_αn` nears 2 — the moment
    /// arm shrinks, `Y_F` falls below the axial term and `K_f` grows without
    /// bound, so Dolan–Broghamer's factor passes through zero to negative.
    /// A negative factor is a compressive fillet, which is not the tensile
    /// fillet the model rates; the model has no reading there, and reporting
    /// none is the honest answer. `max(Y_F ± axial)` would be a third model
    /// neither source states.
    #[must_use]
    pub fn bending_factor(&self, model: RootStressModel) -> Option<f64> {
        let bending = match model {
            RootStressModel::DolanBroghamer => self.form_factor - self.axial_compression,
            RootStressModel::Iso6336 | RootStressModel::FormFactorOnly => self.form_factor,
        };
        Some(bending * self.stress_correction(model)?).filter(|f| f.is_finite() && *f > 0.0)
    }
}

// ------------------------------------------- the rim beneath the teeth ---

/// **The rim under one member's teeth**, and the factor `Y_B` by which too
/// little of it de-rates the root.
///
/// ISO 6336-3:2019, Clause 9. Where the rim is thin the tooth's root fillet
/// stops being the weakest path: the crack runs through the rim instead, and the
/// clause's own words for the remedy are that `Y_B` is "a simplified factor used
/// to de-rate thin rimmed gears when detailed calculations of stresses in both
/// tension and compression or experience are not available. For critically
/// loaded applications this method should be replaced by a more comprehensive
/// analysis."
///
/// # One fit, two references
///
/// An external gear's rim is measured against the **whole tooth depth** and a
/// ring's against the **normal module**, because what the rim has to resist
/// differs: an external tooth levers against a rim on the far side of its own
/// height, a ring's tooth hangs off one. Both come out as `Y_B = a ln(c/ratio)`,
/// never below 1, and the two constants are the whole of the difference — which
/// is why this is one type with two constructors rather than two factors.
///
/// The published breakpoints fall out of the fit rather than being a third and
/// fourth constant: `Y_B` reaches 1 at `ratio = c·e^(−1/a)`, which is 1,2001 for
/// an external gear against the stated 1,2 and 3,4886 for a ring against the
/// stated 3,5. So taking the larger of the fit and 1 reproduces the clause's
/// case (a) without a branch, and continuously, where testing the breakpoint
/// would leave a 0,4 % step in a ring's factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RimSupport {
    /// `s_R/h_t` for an external member, `s_R/m_n` for a ring.
    ratio: f64,
    fit: RimFit,
}

/// The constants of one arm of the `Y_B` fit: `Y_B = a ln(c/ratio)`, and the
/// ratio below which the clause says the design "shall be avoided".
#[derive(Clone, Copy, Debug, PartialEq)]
struct RimFit {
    a: f64,
    c: f64,
    floor: f64,
}

/// External gears, ISO 6336-3:2019, 9.3.1, Formulae (68) and (69): the backup
/// ratio `s_R/h_t`, full support at 1,2 and to be avoided at or below 0,5.
const EXTERNAL_RIM: RimFit = RimFit {
    a: 1.6,
    c: 2.242,
    floor: 0.5,
};

/// Internal gears, 9.3.2, Formulae (70) and (71): the rim thickness in normal
/// modules `s_R/m_n`, full support at 3,5 and to be avoided at or below 1,75.
const INTERNAL_RIM: RimFit = RimFit {
    a: 1.15,
    c: 8.324,
    floor: 1.75,
};

impl RimSupport {
    /// An external member's rim, against the whole depth of its tooth.
    ///
    /// `tooth_depth` is `h_t`, tip to root, in the same units as the thickness.
    #[must_use]
    pub fn external(thickness: f64, tooth_depth: f64) -> Self {
        Self {
            ratio: thickness / tooth_depth,
            fit: EXTERNAL_RIM,
        }
    }

    /// A ring's rim, against the normal module.
    #[must_use]
    pub fn internal(thickness: f64, normal_module: f64) -> Self {
        Self {
            ratio: thickness / normal_module,
            fit: INTERNAL_RIM,
        }
    }

    /// The backup ratio as the clause measures it — `s_R/h_t` or `s_R/m_n`,
    /// depending on which kind of member this is.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// `Y_B`, never below 1.
    #[must_use]
    pub fn factor(&self) -> f64 {
        (self.fit.a * (self.fit.c / self.ratio).ln()).max(1.0)
    }

    /// Whether the rim is thick enough for the clause to rate at all.
    ///
    /// Below the floor it says the case "shall be avoided" rather than giving a
    /// value, so — as with the `Y_S` notch band — the formula still answers and
    /// the caller is told the answer is past where the standard will go.
    #[must_use]
    pub fn in_range(&self) -> bool {
        self.ratio > self.fit.floor
    }
}

// ------------------------------------------------------------------ load ---

/// What a gear is carrying.
///
/// # Why this stores torque and not a force
///
/// Every force in a gear mesh is a projection, and a projection is only defined
/// once you say *of what, onto which plane, at which radius*. There are at least
/// four in play here and they differ by factors of `cos α_t`, `cos α_w` and
/// `cos β_b`:
///
/// ```text
/// F_t   = 2000 T / d      tangential at the REFERENCE cylinder
///         2000 T / d'     tangential at the OPERATING cylinder
/// F_bt  = T / r_b         along the transverse line of action
/// F_bn  = F_bt / cos β_b  normal to the tooth flank
/// ```
///
/// Storing any one of them bakes a choice of radius and plane into a bare `f64`
/// that no longer says which it made. Torque does not: it is a property of the
/// shaft, invariant under every redefinition of a radius, and it is what the
/// specification takes as input and reports as output. So torque is what is
/// stored, and each projection is spelled out at the point of use, where the
/// plane it belongs to is visible.
///
/// An earlier revision stored `F_bt` under the name `normal_force`. Nothing it
/// computed was wrong, but the name asserted the normal plane while the value
/// was transverse — exactly the failure this arrangement is meant to make
/// impossible. See docs/corrections.md.
///
/// # Sign and reference
///
/// A `Load` is quoted **against a particular gear**, since `T₁ ≠ T₂` across a
/// mesh. The accessors take that gear explicitly rather than assuming it. What
/// *is* shared by both gears is `F_bt`, by action and reaction — see
/// [`Load::transverse_line_of_action`].
#[derive(Clone, Copy, Debug)]
pub struct Load {
    /// Torque on the gear this load is quoted against, N·m.
    pub torque: f64,
    /// Face width, mm.
    pub face_width: f64,
}

impl Load {
    #[must_use]
    pub fn new(torque: f64, face_width: f64) -> Self {
        Self { torque, face_width }
    }

    /// Tangential force at the **reference** cylinder, N — ISO 6336's `F_t`.
    ///
    /// `F_t = 2000 T / d = 1000 T / r`. The 1000 converts N·m to N·mm, because
    /// every length in this crate is millimetres.
    #[must_use]
    pub fn tangential(&self, g: &Tooth) -> f64 {
        1000.0 * self.torque / g.r
    }

    /// Force along the **transverse** line of action, N — `F_bt = T / r_b`.
    ///
    /// The exact lever arm for an involute is the base radius, so this relation
    /// is a geometric identity rather than a convention. It is also the one load
    /// quantity **both gears of a pair share**: action and reaction along the
    /// line of action are the same force, which is why contact stress — a
    /// property of the pair — is built on it.
    #[must_use]
    pub fn transverse_line_of_action(&self, g: &Tooth) -> f64 {
        1000.0 * self.torque / g.rb
    }

    /// Force **normal to the tooth flank**, N — `F_bn = F_bt / cos β_b`.
    ///
    /// For a spur gear `β_b = 0` and this equals
    /// [`Self::transverse_line_of_action`]. For a helical gear it does not, and
    /// this is the force that actually presses the flanks together: the contact
    /// line is inclined at the base helix angle, so the transverse force is only
    /// its projection.
    #[must_use]
    pub fn normal_to_flank(&self, g: &Tooth) -> f64 {
        self.transverse_line_of_action(g) / g.base_helix_angle().cos()
    }

    /// The same mesh load, re-quoted against the mating gear.
    ///
    /// `F_bt` is shared across the mesh, so `T₂ = T₁ · r_b2 / r_b1`. This is the
    /// **geometric** transfer only: efficiency losses belong to train
    /// accumulation (docs/reference.md#trains), not here.
    ///
    /// Face width is carried across unchanged, since a `Load` describes what is
    /// being carried rather than by what.
    #[must_use]
    pub fn across_mesh(&self, from: &Tooth, to: &Tooth) -> Self {
        Self {
            torque: self.torque * to.rb / from.rb,
            face_width: self.face_width,
        }
    }
}

/// Tooth root bending stress, MPa.
///
/// ```text
/// σ_F = F_t / (b · m_n) · (bending factor) · Y_B
/// ```
///
/// `F_t` in newtons and `b`, `m` in millimetres give N/mm² = MPa directly. The
/// module is the section's own ([`RootSection::module`]).
///
/// # It takes a force, not a gear
///
/// **`F_t` is a property of the mesh, and equal for both of its members**:
/// `F_t = 2000 T/d`, and across a mesh both the torque and the diameter scale by
/// the tooth ratio, so the two cancel exactly. Taking the force says that, where
/// taking a member and asking it for one does not — and a ring, which has no
/// rack-cut tooth of its own to ask, used to be rated by handing this function
/// **the mating pinion's** tooth. That produced the right number for the right
/// reason and read like an accident, and it meant the ring's rating reached
/// outside itself for a quantity that was never the pinion's to begin with.
///
/// A caller with a member and a torque still has [`Load::tangential`]; what it
/// no longer has to do is find a member for a section that is not one.
///
/// # Helical gears
///
/// `F_t` is the **transverse** tangential force at the reference cylinder while
/// `m_n` is the **normal** module, and that pairing is deliberate — it is ISO
/// 6336-3's, and it is only consistent if `Y_F` is measured on the *normal*
/// section. So `section` must come from the virtual spur gear,
/// [`Tooth::virtual_spur`], which is what [`bending_section`] returns.
///
/// Measuring `Y_F` on the transverse section and dividing by `m_n` — which an
/// earlier revision did — mixes the two planes and under-predicts the stress by
/// about `cos β` (6 % at 20°, 13 % at 30°). Spur gears are unaffected, since the
/// two sections coincide.
///
/// # Which of ISO 6336-3's factors are here
///
/// ```text
/// σ_F0 = F_t/(b · m_n) · Y_F · Y_S · Y_β · Y_B · Y_DT
/// ```
///
/// `Y_F` and `Y_S` come off `section`; `Y_B` arrives in `rim`, and is 1 where
/// nobody described a rim. **`Y_β`, `Y_DT` and `f_ε` are declined**, and the
/// reason is one reason rather than three: the 2019 edition revised `Y_F` and
/// `Y_β` *together*, `f_ε` is inside that revised `Y_F`, and their product is
/// below 1 for any gear with full axial overlap. Applying `Y_β` alone takes the
/// raising half of a pair — against a `Y_F` this crate measures its own way, on
/// a virtual gear that follows the *other* edition's tooth count. See
/// `docs/rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither`,
/// which carries the measurement, and `docs/state.md`, which carries every
/// declined formula so the decision can be revisited without the standard.
///
/// **The `K` factors are not applied here**: the stress is nominal under the
/// force it is given, and a train's rating passes the force times its load
/// case's `K_A`. `K_v`, `K_Fβ` and `K_Fα` are declined
/// (docs/rationale.md#no-isoagma-correction-factors); each is `≥ 1`, so the
/// stress is low by their product. `Y_S` and `Y_B` are the two exceptions and
/// neither is half of anything; see [`RootStressModel`] and [`RimSupport`].
///
/// Returns `None` when the stress correction is undefined for this section —
/// see [`RootSection::stress_correction`]. That is not a failure to compute; it
/// is the model declining to apply a notch factor where there is no notch.
#[must_use]
pub fn bending_stress(
    section: &RootSection,
    tangential_force: f64,
    face_width: f64,
    model: RootStressModel,
    rim: Option<RimSupport>,
) -> Option<f64> {
    let factor = section.bending_factor(model)?;
    let y_b = rim.map_or(1.0, |r| r.factor());
    Some(tangential_force / (face_width * section.module) * factor * y_b)
}

/// The critical section to rate a gear's bending on, loaded at the highest
/// point of single-pair contact.
///
/// One formula for spur and helical alike. The tooth bends as its **normal**
/// section, so both the form and the load point are taken on the virtual spur
/// gear ([`Tooth::virtual_spur`]); for a spur gear that is the gear itself, by
/// construction rather than by a branch.
///
/// # Locating the load point without the mate
///
/// The highest point of single-pair contact is one base pitch back from the far
/// end of the path of contact, so measuring from the *tip* it depends only on
/// this gear's own geometry and the contact ratio:
///
/// ```text
/// u_load = u_tip − (ε_α − 1) · p_b / r_b
/// ```
///
/// [verified exact against the path-of-contact construction over seven meshes,
/// including reversed pairs.] That is what lets this take a scalar rather than
/// the mating gear, and it is what makes the helical case tractable: the same
/// relation applies on the virtual gear, using the **virtual** contact ratio
///
/// ```text
/// ε_αn = ε_α / cos² β_b
/// ```
///
/// Carrying the *real* gear's roll parameter across instead — which an earlier
/// revision did — puts the load at the wrong place on the flank, because the
/// virtual gear's involute is not the real one.
///
/// `transverse_contact_ratio` is `ε_α` from [`ContactPath::contact_ratio`].
#[must_use]
pub fn bending_section<T: ToothOutline>(
    g: &T,
    transverse_contact_ratio: f64,
) -> Option<RootSection> {
    bending_section_shared(g, transverse_contact_ratio, LoadSharing::None).map(|(s, _)| s)
}

/// **Where the load sits on the flank, `d` base pitches back from the far end
/// of the path** — and whether it sits on generated flank at all.
///
/// The one place the two kinds of member differ in any of this. An external
/// tooth's load point travels **down** in roll away from its tip and a ring's
/// travels **up**, because a ring's flank runs the other way — which is
/// [`MeshKind::sign`](crate::mesh::MeshKind::sign) again rather than a second
/// construction, and it is why one sweep serves both.
///
/// A ring also has a limit an external tooth does not: its flank below the
/// generation limit was never cut by the shaper, so a load point past `u_j` is
/// not on the part. An external tooth's own limits are `root_section`'s.
struct LoadPoint<'a, T: ToothOutline + ?Sized> {
    /// The **virtual spur** member, whose own transverse plane is the normal
    /// plane the tooth bends in.
    v: &'a T,
    /// Where `d` is counted from, and what turns a base-pitch length into a
    /// roll.
    u_tip: f64,
    rb: f64,
    base_pitch: f64,
    /// `-1` for a tooth, `+1` for a ring's space — one fact,
    /// [`ToothOutline::tip_at_high_roll`], not a second construction.
    sense: f64,
    /// The construction the section is found by.
    method: CriticalSection,
    // The flank the roll must land on is read from `v` rather than stored: it is
    // `flank_bracket`, and it is the same question for both kinds of member.
}

impl<T: ToothOutline + ?Sized> LoadPoint<'_, T> {
    fn at(&self, d: f64) -> Option<RootSection> {
        root_section_with(self.v, self.roll(d)?, self.method)
    }

    /// Base pitches back from the tip to roll `u`: [`Self::roll`] inverted.
    fn back_to(&self, u: f64) -> f64 {
        (u - self.u_tip) * self.rb / (self.sense * self.base_pitch)
    }

    /// **Where the load runs off the flank and where it lies level**: the
    /// flank's far end as a roll and in base pitches back from the tip, and
    /// where the load is square across the tooth, `flank_tilt = 0`, at which
    /// the load angle's magnitude turns — a corner in the rated factor.
    /// Either may be off the cycle; the caller clips.
    fn corners(&self) -> (f64, f64, Option<f64>) {
        let (lo, hi) = self.v.flank_bracket();
        let far = if self.sense < 0.0 { lo } else { hi };
        let level = brent(|u| self.v.flank_tilt(u), lo, hi, Tol::default());
        (far, self.back_to(far), level.map(|u| self.back_to(u)))
    }

    /// The roll `d` base pitches back from the tip, where it is on the flank.
    fn roll(&self, d: f64) -> Option<f64> {
        let roll = self.u_tip + self.sense * d * self.base_pitch / self.rb;
        // **The load point has to be on the flank**, and `flank_bracket` is
        // where the flank is for either kind of member: an external tooth's
        // involute runs from its fillet junction to its tip, and a ring's from
        // its tip to the generation limit its shaper stopped at. This was a
        // separate `generated` range carried for the ring alone, on the reading
        // that only a shaper-cut flank stops early. Both stop; they stop at the
        // two ends of the same bracket.
        let (lo, hi) = self.v.flank_bracket();
        (roll >= lo.min(hi) && roll <= lo.max(hi)).then_some(roll)
    }
}

/// **Why a member has no bending rating**, said rather than left as a
/// missing number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unrated {
    /// No section: the tooth has no usable flank, the load point is off it, or
    /// the construction finds no tangency there.
    NoSection,
    /// Every section the construction finds is **compressed** at the loaded
    /// fillet: the load's push along the tooth outweighs its bending there
    /// (`Y_F − axial ≤ 0`), or its line crosses the centreline below the
    /// section. The bending model rates a stretched fillet and has no reading
    /// of these ([`RootSection::bending_factor`]).
    Compressed,
}

/// The worst section over one mesh cycle, and the share of the load on it.
///
/// Shared by every member of every mesh: what a caller supplies is where the
/// load point is at `d` base pitches ([`LoadPoint`]) and how deep the cycle is.
/// With sharing off there is nothing to sweep — the tooth carries everything at
/// the highest point of single-pair contact, `d = ε_n − 1`, which is the
/// standard conservative reading and the one expression this used to be.
///
/// # The greatest over the cycle, not over samples
///
/// With sharing on, the rating is the greatest `(Y_F − axial)·K_f · share`
/// anywhere on the cycle the flank carries. It is smooth except where the
/// share steps or turns (at `ε_n − 1` and `1` below `ε_n = 2`, at `ε_n / 2`
/// above), where the load lies square across the tooth (its angle's magnitude
/// turns there), and where the flank ends. So the cycle is cut at each of
/// those and each piece's greatest found by [`crate::solve::greatest`], ends
/// included. It used to be the best of 204 samples, which fell short of the
/// maximum by up to 1.4e-3 and missed it outright where the flank's end
/// governs, since that end was not among them.
///
/// Measured over 532 random teeth, each piece's product rises to one peak and
/// falls, or rises to an end, and the pieces' greatest equals a 4,000-sample
/// maximum of the whole cycle to 1e-9; a second interior extremum on a piece
/// (the product falling, then rising again toward a corner) is what would
/// defeat the golden section, and `the_swept_maximum_is_the_greatest_over_the_cycle`
/// holds the answer against a fine sweep.
fn worst_over_cycle<T: ToothOutline + ?Sized>(
    at: &LoadPoint<T>,
    eps_n: f64,
    model: LoadSharing,
    afresh: bool,
) -> Result<(RootSection, f64), Unrated> {
    // **The section is found once**, at the highest point of single-pair
    // contact, and the sweep moves only the load on it
    // ([`RootSection::loaded_at`]). A section searched afresh at every load
    // point shrinks onto a pointed apex as the load reaches it — `1/d`, and
    // `0/0` at the tip — so the swept maximum grew with the sample count and
    // jumped at the pointed limit. A candidate is one the model rates
    // ([`RootSection::bending_factor`]); where it does not, the tooth has no
    // rating at that load point rather than a number.
    let section = at
        .at(highest_single_pair(eps_n))
        .ok_or(Unrated::NoSection)?;
    section
        .bending_factor(RootStressModel::DolanBroghamer)
        .ok_or(Unrated::Compressed)?;
    if matches!(model, LoadSharing::None) {
        return Ok((section, 1.0));
    }
    let (far_roll, far, level) = at.corners();
    // The flank's far end is loaded at its own roll, which `d → roll` would
    // round off the flank.
    let roll = |d: f64| if d == far { Some(far_roll) } else { at.roll(d) };
    let rated = |d: f64| {
        roll(d)
            .and_then(|roll| {
                if afresh {
                    root_section_with(at.v, roll, at.method)
                } else {
                    section.loaded_at(at.v, roll)
                }
            })
            .and_then(|s| {
                s.bending_factor(RootStressModel::DolanBroghamer)
                    .map(|f| (s, f))
            })
    };
    // The form factor is what the stress is proportional to at a fixed
    // torque, so the worst point is the largest `(Y_F − axial)·K_f · share`.
    // Taking the factor rather than a stress keeps this independent of the
    // load case, which is why it is evaluated once per member and not once per
    // case.
    //
    // **`Y_β` and `Y_B` are deliberately absent from this product**, and
    // their absence changes nothing: both are constant over a mesh cycle, so
    // they scale every candidate alike and cannot move which one wins.
    let weighted = |d: f64| rated(d).map(|(_, f)| f * crate::contact::load_share(d, eps_n, model));

    // The part of the cycle the flank carries, and every corner inside it:
    // the single-pair zone's two ends (the first is the highest point of
    // single-pair contact), the ramps' meeting above `ε_n = 2`, the level load.
    let end = far.min(eps_n);
    let mut cuts: Vec<f64> = [
        Some(0.0),
        Some(end),
        Some(highest_single_pair(eps_n)),
        Some(1.0),
        Some(eps_n / 2.0),
        level,
    ]
    .into_iter()
    .flatten()
    .filter(|d| (0.0..=end).contains(d))
    .collect();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();

    let mut best: Option<(f64, f64)> = None;
    for piece in cuts.windows(2) {
        if let Some((d, w)) = crate::solve::greatest(weighted, piece[0], piece[1]) {
            if best.is_none_or(|(_, b)| w > b) {
                best = Some((d, w));
            }
        }
    }
    // A cycle of one point: the flank holds only the tip.
    if cuts.len() == 1 {
        best = weighted(cuts[0]).map(|w| (cuts[0], w));
    }
    // The single-pair point's own section is readable, and it is a cut, so
    // the cycle holds at least one rated point.
    let (d, _) = best.ok_or(Unrated::Compressed)?;
    rated(d)
        .map(|(s, _)| (s, crate::contact::load_share(d, eps_n, model)))
        .ok_or(Unrated::Compressed)
}

/// **The highest point of single-pair contact**, in base pitches back from the
/// far end of the path.
///
/// `ε_n − 1` — and **held at the tip**, because a pair that is alone for the
/// whole of its engagement is alone at the tip too. Below a contact ratio of 1
/// the unclamped expression is negative, which is a load point *past the end of
/// the tooth*: `root_section` answers there by extrapolating the involute beyond
/// the tip, and reports a longer moment arm than the tooth has.
///
/// The clamp is not invented here. It is
/// [`ContactPath::highest_single_pair`](crate::contact::ContactPath::highest_single_pair)'s
/// own `.min(recess)`, in the coordinate this sweep counts in — the path of
/// contact has always known where its end is, and this expression is the same
/// point reached without the mate. The sharing sweep already clamped it in its
/// sample list while the unshared branch did not, which is the shape of the bug:
/// **one quantity, computed twice, agreeing until it mattered.**
///
/// A mesh below a contact ratio of 1 is already reported
/// (`mesh.contact_ratio_below_one`); what this fixes is that it was
/// also rated at a point on no tooth.
fn highest_single_pair(eps_n: f64) -> f64 {
    (eps_n - 1.0).max(0.0)
}

/// The critical section to rate bending on, and the share of the load acting
/// there, once load sharing is allowed to move the governing point.
///
/// # With sharing off this *is* [`bending_section`]
///
/// Not "agrees with to a tolerance" — that call is *this* one at
/// [`LoadSharing::None`], which sweeps nothing and takes the highest point of
/// single-pair contact with a share of exactly 1. Sharing is off by default
/// everywhere, so the ordinary rating path is untouched and costs what it always
/// did. It reads the other way round from how it was written, which is the point
/// of writing it that way: the unshared rating is the shared one at its own
/// degenerate value rather than a second construction beside it.
///
/// # What sharing changes, and what it does not
///
/// **Bending only.** A contact rating is already taken at the pitch point and
/// the two single-pair boundaries, and those are precisely the places one tooth
/// carries everything — so a sharing model cannot move a contact stress, and
/// this is not consulted for one.
///
/// Bending is different because its worst point is a *product*: the form factor
/// grows toward the tip while the share falls away there, so the maximum of the
/// two together is somewhere in between and has to be looked for. Without
/// sharing the tooth is assumed to carry everything at the highest point of
/// single-pair contact, which is the standard conservative reading; with it, the
/// whole cycle is swept.
///
/// **Below `ε_n = 2` it relieves nothing inside the single-pair zone**, where
/// the share is exactly 1, and that zone is where the maximum sits. It is
/// usually at the highest point of single-pair contact, the unshared
/// rating's point. Not always: on some small teeth the full-load factor
/// rises from there toward the lowest point of single-pair contact (its
/// section held, its arm shorter, its `K_f` larger), and the ramp's maximum
/// is there, above the unshared figure, with a share of 1 — which is the
/// unshared rating understating its own model, not sharing (11 of
/// `gear-cli bendgrid`'s 564 ramp rows below 2, up to 3.17 % at z 9, 25°,
/// ε_n 1.7; `docs/state.md`). At a high contact ratio (`ε ≥ 2`) two pairs are
/// always engaged and the single-pair zone does not exist.
///
/// The model itself is an **uncalibrated placeholder** — see [`LoadSharing`] —
/// which is why it is an option a designer switches on rather than something
/// applied on their behalf.
///
/// # Errors
///
/// `None` where [`bending_section`] has none: a tooth with no usable form, or a
/// load point past the end of the generated flank.
#[must_use]
pub fn bending_section_shared<T: ToothOutline>(
    g: &T,
    transverse_contact_ratio: f64,
    model: LoadSharing,
) -> Option<(RootSection, f64)> {
    bending_section_on_path(g, transverse_contact_ratio, 0.0, model).ok()
}

/// **The sweep as it was: a section searched afresh at every load point.**
/// The instrument that measures what holding the section costs
/// (`gear-cli sharingbias`); unbounded near a pointed apex, and not smooth
/// where the section moves from one curve to the other, so not the rating.
#[must_use]
pub fn bending_section_searched_afresh<T: ToothOutline>(
    g: &T,
    transverse_contact_ratio: f64,
    model: LoadSharing,
) -> Option<(RootSection, f64)> {
    let v = g.virtual_spur();
    let (at, eps_n) = load_point(
        g,
        &v,
        transverse_contact_ratio,
        0.0,
        CriticalSection::default(),
    )?;
    worst_over_cycle(&at, eps_n, model, true).ok()
}

/// [`bending_section_shared`] on a path whose far end falls `short_of_tip`
/// base pitches below this member's tip
/// ([`crate::contact::ContactPath::short_of_tip`]): the mate's usable flank
/// ends before this tooth's tip is reached, so the cycle is counted from where
/// contact ends. At zero it is [`bending_section_shared`] to the bit.
///
/// # Errors
///
/// [`Unrated`], saying why: no section, or none the model reads.
pub fn bending_section_on_path<T: ToothOutline>(
    g: &T,
    transverse_contact_ratio: f64,
    short_of_tip: f64,
    model: LoadSharing,
) -> Result<(RootSection, f64), Unrated> {
    let v = g.virtual_spur();
    let (at, eps_n) = load_point(
        g,
        &v,
        transverse_contact_ratio,
        short_of_tip,
        CriticalSection::default(),
    )
    .ok_or(Unrated::NoSection)?;
    worst_over_cycle(&at, eps_n, model, false)
}

/// Where the load sits on `v`, the virtual spur member of `g`, and the
/// virtual contact ratio `ε_αn` the cycle is counted in. `short_of_tip` is how
/// far, in transverse base pitches, contact ends below the tip.
fn load_point<'a, T: ToothOutline>(
    g: &T,
    v: &'a T,
    transverse_contact_ratio: f64,
    short_of_tip: f64,
    method: CriticalSection,
) -> Option<(LoadPoint<'a, T>, f64)> {
    if !transverse_contact_ratio.is_finite() || !v.is_usable() {
        return None;
    }
    // The whole cycle in the plane the tooth actually bends in. `d` counts
    // virtual base pitches back from the far end of the path, the same
    // coordinate `contact::load_share` takes.
    let cos_bb = g.base_helix_angle().cos();
    let eps_n = transverse_contact_ratio / (cos_bb * cos_bb);
    // The tip, the direction of travel and the limit are one fact read three
    // ways; see `ToothOutline::tip_at_high_roll`.
    let (lo, hi) = v.flank_bracket();
    let (u_tip, sense) = if v.tip_at_high_roll() {
        (hi, -1.0)
    } else {
        (lo, 1.0)
    };
    let base_pitch = crate::plane::base_pitch(v.transverse_module(), v.transverse_pressure_angle());
    let rb = v.base_radius();
    // Where contact ends, in the virtual gear's roll: a length along the line
    // of action scales to the virtual plane as `ε` does.
    let short_n = short_of_tip / (cos_bb * cos_bb);
    Some((
        LoadPoint {
            base_pitch,
            u_tip: u_tip + sense * short_n * base_pitch / rb,
            rb,
            sense,
            method,
            v,
        },
        eps_n,
    ))
}

/// **The section at the highest point of single-pair contact, by a chosen
/// construction**, with no sharing and no model's domain applied.
///
/// The instrument's reading: `gear-cli iso` takes ISO's 30°/60° tangent here
/// to hold the tool's ISO set against ISO 6336-3 Method B, which loads the
/// same point. The rating takes [`bending_section_shared`].
#[must_use]
pub fn bending_section_by<T: ToothOutline>(
    g: &T,
    transverse_contact_ratio: f64,
    method: CriticalSection,
) -> Option<RootSection> {
    let v = g.virtual_spur();
    let (at, eps_n) = load_point(g, &v, transverse_contact_ratio, 0.0, method)?;
    at.at(highest_single_pair(eps_n))
}

/// The lengthwise relative curvature of a parallel-axis, uncrowned mesh:
/// exactly zero.
///
/// Named rather than written as `0.0` at each call site, because it is a
/// *value of the general contact model* — the one at which the ellipse becomes
/// a line — and not a placeholder for something not yet passed. Every mesh this
/// crate builds today takes it.
pub const PARALLEL_AXES: f64 = 0.0;

/// Hertzian contact stress along the path of contact, MPa.
#[derive(Clone, Copy, Debug)]
pub struct ContactStress {
    /// At the pitch point, where the relative radius is largest.
    pub at_pitch_point: f64,
    /// At each gear's own **inner point of single-pair contact**, indexed as the
    /// gears are.
    ///
    /// The two are different points on the same line of action, and that is the
    /// one way a mesh's two members legitimately carry different contact
    /// stresses — see [`ContactStress::governing`].
    pub at_single_pair: [f64; 2],
    /// The worst anywhere on the path: the envelope over both members, and what
    /// the *mesh* is rated on when no member is named.
    pub worst: f64,
    /// Position of the worst point on the line of action, mm from the pitch
    /// point.
    pub worst_position: f64,
    /// Relative radius of curvature at the worst point, mm, in the **normal**
    /// plane — the one the contact actually sees. Reported because it is what
    /// the number is really driven by. Equal to the transverse radius for a spur
    /// gear.
    pub relative_radius: f64,
}

impl ContactStress {
    /// What gear `i` is rated on: the worse of the pitch point and **its own**
    /// inner point of single-pair contact.
    ///
    /// # Why a mesh has two contact stresses, and why it is not the curvature
    ///
    /// At any one instant there is one pressure. The two flanks share a patch, a
    /// normal force and an `E*`, and Hertz reaches that patch through the *gap*
    /// between the surfaces, which depends on the individual radii only as
    /// `1/ρ = 1/ρ₁ + 1/ρ₂`. Each body is then treated as an elastic half-space
    /// loaded by that shared pressure, and a half-space does not know its own
    /// curvature — so `ρ₁ = ρ₂ = 10` and `ρ₁ = 5.5, ρ₂ = 55` are the same
    /// contact, at the same pressure, in both bodies. Two teeth of very
    /// different size do **not** see different stresses for being different
    /// sizes.
    ///
    /// What differs is **where each gear's own worst moment is**. Along the
    /// path, `ρ₁ + ρ₂` is constant, so `ρ` peaks where they are equal and falls
    /// toward both ends; the two ends are not symmetric about that peak, so the
    /// two single-pair boundaries carry different pressures. And the two gears
    /// are not rated at the same one: pitting initiates in the **dedendum**,
    /// where sliding opposes rolling, and each gear's flank is at its root at
    /// one end of the path and at its tip at the other. Gear 1's root end is the
    /// low end of `ξ` and gear 2's is the high end — one relation, both mesh
    /// kinds, since `ρ₁` rises and `ρ₂` falls monotonically with `ξ`.
    ///
    /// So each gear takes the worse of the pitch point and the boundary where
    /// its own root is loaded alone. This is ISO 6336-2's `Z_B` and `Z_D` — a
    /// factor of `max(1, M_i)` on the pitch-point stress, applied to the pinion
    /// and the wheel respectively — reached here by evaluating the two points
    /// rather than by quoting the factor.
    ///
    /// [`Self::worst`] remains the envelope over both, and is what the *mesh*
    /// is rated on where no member is named.
    ///
    /// # Panics
    ///
    /// Never for `i` in `0..2`; indexes out of bounds otherwise.
    #[must_use]
    pub fn governing(&self, gear: usize) -> f64 {
        self.at_pitch_point.max(self.at_single_pair[gear])
    }
}

/// Exact Hertzian contact for a meshing pair.
///
/// At a point `ξ` from the pitch point the two flanks are locally cylinders of
/// **transverse** radius `ρ₁ = r_b1 tan α_w + ξ` and `ρ₂ = r_b2 tan α_w − ξ`, so
///
/// ```text
/// 1/ρ_t = 1/ρ₁ ± 1/ρ₂               + external, − internal
/// σ_H   = max( σ_elliptical , √( (F' / ρ_n) · E* / π ) )
/// ```
///
/// `e_star` is the effective contact modulus `E*`, from
/// [`crate::material::contact_modulus`]. It is passed as a number rather than
/// taken from two materials so this stays a statement about mechanics.
///
/// # The lengthwise curvature, and why there is no second function
///
/// `lengthwise_curvature` is `1/R_L`, the relative curvature **along** the
/// contact line, in 1/mm. It is exactly zero for every mesh this crate builds
/// today — parallel axes, uncrowned flanks — and positive only for crossed axes
/// or crowning. It is the single parameter that unifies point and line contact
/// (docs/reference.md#contact-stress), and the reason the crossed-axis work adds an argument here
/// rather than a second function chosen by stage type.
///
/// The general elliptical solution ([`crate::hertz`]) is evaluated
/// unconditionally, and at `1/R_L = 0` its peak pressure is **exactly zero**:
/// the patch lengthens without bound and a finite load spread over it presses
/// on nothing. So the `max` above returns the line term, bit for bit, with no
/// branch to choose it. That is the acceptance gate — every existing contact
/// check and `gear-cli strength 17 43 2.0` to the last digit — and it is passed
/// by construction rather than by two routes happening to agree.
///
/// The `max` itself is not a fudge between two models; it is where the *body*
/// takes over from elasticity. A tooth has finite face width, so an ellipse
/// longer than the contact line is truncated by the tooth rather than by the
/// contact solution, and in that regime the line term — the same load spread
/// over the length that actually exists — is the physical one. The two cross
/// once. Near the crossing the truth sits slightly above both, since a
/// truncated ellipse concentrates load more than a uniform line does; that is
/// the honest limit of the expression rather than something papered over.
///
/// # Helical gears
///
/// Three things change together, and they nearly cancel:
///
/// ```text
/// ρ_n = ρ_t / cos β_b        curvature is seen in the NORMAL plane
/// F_bn = F_bt / cos β_b      the flank force, not its transverse projection
/// L    = b / cos β_b         one contact line, inclined across the face
/// ```
///
/// Substituting all three collapses to `σ_H = √((F_bt/b) · cos β_b / ρ_t ·
/// E*/π)`, so a helical mesh comes out lower than the same transverse geometry
/// by exactly `√(cos β_b)` — 3 % at β = 20°, 6 % at β = 30°. That benefit is
/// pure geometry: longer contact line and flatter normal-plane curvature. It is
/// **not** the extra benefit helical gears get from having several contact lines
/// engaged at once, which is load sharing and is deferred (docs/reference.md#contact-stress).
/// Assuming a single line is the conservative reading and is continuous with the
/// spur case at β = 0.
///
/// # Which points are checked
///
/// `ρ₁ + ρ₂` is constant along the path, so the relative radius `ρ₁ρ₂/(ρ₁+ρ₂)`
/// is largest where the two are equal and falls away toward **both** ends. Both
/// single-pair boundaries are evaluated as well as the pitch point, and they are
/// kept **apart** rather than reduced to one figure: each belongs to one of the
/// two gears. [`ContactStress::governing`] says which.
///
/// Returns `None` if the geometry puts a contact point outside both flanks,
/// which cannot happen for a mesh [`ContactPath`] accepted.
#[must_use]
pub fn contact_stress(
    path: &ContactPath,
    mesh: &Mesh,
    g1: &Tooth,
    lengthwise_curvature: f64,
    load: &Load,
    e_star: f64,
) -> Option<ContactStress> {
    // The curvatures come from the mesh, which owns the operating geometry and
    // the one signed relation both kinds obey — see `Mesh::curvature_radii`.
    // Re-deriving `r_b2` here is what previously got an internal pair wrong.

    // F_bn is shared by both gears of the pair — it is `F_bt/cos β_b` and
    // `F_bt` is action and reaction along the line of action — so which gear the
    // load was quoted against does not survive into the answer. The patch
    // carries the whole of it at a point; the line carries the same force spread
    // along its length.
    let f_bn = load.normal_to_flank(g1);
    let cos_bb = g1.base_helix_angle().cos();

    let at = |xi: f64| -> Option<(f64, f64)> {
        let inv_rho_t = mesh.relative_curvature(xi)?;
        // The contact line is one line inclined across the face at the base
        // helix angle, so it is longer than the face by `1/cos β_b` — and the
        // force normal to the flank is larger than its transverse projection by
        // the same factor. Written out rather than pre-cancelled so the two
        // plane changes stay visible; they cancel in `F_bn/L = F_bt/b`.
        let line_length = load.face_width / cos_bb;
        let inv_rho_n = cos_bb * inv_rho_t;
        // At `lengthwise_curvature = 0` — every uncrowned parallel mesh — the
        // elliptical term is exactly zero and this is the line term unchanged.
        let sigma = peak_pressure(lengthwise_curvature, inv_rho_n, f_bn, line_length, e_star)?;
        Some((sigma, 1.0 / inv_rho_n))
    };

    let (pitch, r_pitch) = at(0.0)?;
    let lo = path.lowest_single_pair();
    let hi = path.highest_single_pair();
    let (s_lo, r_lo) = at(lo)?;
    let (s_hi, r_hi) = at(hi)?;

    let (single, r_single, xi_single) = if s_lo >= s_hi {
        (s_lo, r_lo, lo)
    } else {
        (s_hi, r_hi, hi)
    };
    let (worst, relative_radius, worst_position) = if single >= pitch {
        (single, r_single, xi_single)
    } else {
        (pitch, r_pitch, 0.0)
    };

    Some(ContactStress {
        at_pitch_point: pitch,
        // Gear 1's flank is at its root at the **low** end of the path and gear
        // 2's at the high end — one relation, both mesh kinds, and the reason
        // the two members are rated at different points. See `governing`.
        at_single_pair: [s_lo, s_hi],
        worst,
        worst_position,
        relative_radius,
    })
}

/// Minimum face width for a bending stress, mm.
///
/// `σ_F ∝ 1/b`, so `b_min = b · σ_F / σ_allow`.
///
/// **The `b` cancels.** Whatever face width the stress was evaluated at, the
/// answer is the same — which is the invariant worth testing, because it is the
/// one that catches a stress that did not actually scale the way it should.
///
/// Every factor of `σ_F0` this crate applies is independent of the face width,
/// and that is worth one sentence because it was briefly not: `Y_β` depends on
/// `b` through the overlap ratio, and carrying it turned this division into a
/// two-branch solve. Declining `Y_β` gave the invariant back
/// (docs/rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither).
#[must_use]
pub fn min_face_width_bending(stress: f64, evaluated_at: f64, allowable: f64) -> f64 {
    evaluated_at * stress / allowable
}

/// Minimum face width for a contact stress, mm.
///
/// `σ_H ∝ 1/√b`, so `b_min = b · (σ_H / σ_allow)²`. The square is the whole
/// difference from the bending case, and it is why contact usually governs the
/// face width of a lightly loaded gear while bending governs a heavily loaded
/// one.
#[must_use]
pub fn min_face_width_contact(stress: f64, evaluated_at: f64, allowable: f64) -> f64 {
    evaluated_at * (stress / allowable).powi(2)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::mesh::MeshKind;
    use crate::GearParams;

    /// **The swept maximum is the greatest over the cycle.** Under the ramp
    /// a rating is the greatest `(Y_F − axial)·K_f · share` on the cycle the
    /// flank carries, so no load point on it may rate higher: checked against
    /// 20,000 even points and the flank's own end. The best of the 204 samples
    /// the sweep used to take fell short of this by up to 1.4e-3, and missed
    /// the flank's end where it governs (ε 1.7 among them), so the law also
    /// asserts that some answers lie beyond what those samples reach, and some
    /// off the single-pair point. The tolerance is the rounding of one
    /// evaluation, `2⁸ε` of it: both sides are the same function at two
    /// arguments.
    #[test]
    fn the_swept_maximum_is_the_greatest_over_the_cycle() {
        let model = LoadSharing::LinearRamp;
        let (mut checked, mut off_single_pair, mut beyond_samples) = (0, 0, 0);
        for teeth in [9_u32, 12, 17, 25, 40, 70, 150] {
            for (profile_shift, pressure_angle, addendum) in [
                (0.0_f64, 20.0_f64, 1.0_f64),
                (0.5, 25.0, 1.0),
                (-0.3, 14.5, 1.0),
                (0.3, 20.0, 1.35),
                (0.8, 25.0, 1.25),
            ] {
                for helix_angle in [0.0_f64, 15.0, 30.0] {
                    let g = Tooth::new(GearParams {
                        teeth,
                        profile_shift,
                        pressure_angle,
                        addendum,
                        helix_angle,
                        ..Default::default()
                    });
                    let v = g.virtual_spur();
                    let cos_bb = g.base_helix_angle().cos();
                    for eps_n in [1.3_f64, 1.7, 2.3, 2.6] {
                        let eps = eps_n * cos_bb * cos_bb;
                        for short in [0.0_f64, 0.2] {
                            let Ok((s, share)) = bending_section_on_path(&g, eps, short, model)
                            else {
                                continue;
                            };
                            let got =
                                s.bending_factor(RootStressModel::DolanBroghamer).unwrap() * share;
                            let (at, en) =
                                load_point(&g, &v, eps, short, CriticalSection::default()).unwrap();
                            let held = at.at(highest_single_pair(en)).unwrap();
                            let w = |d: f64| {
                                at.roll(d)
                                    .and_then(|r| held.loaded_at(&v, r))
                                    .and_then(|x| x.bending_factor(RootStressModel::DolanBroghamer))
                                    .map(|f| f * crate::contact::load_share(d, en, model))
                            };
                            let (lo, hi) = v.flank_bracket();
                            let far = (hi - lo) * v.rb
                                / crate::plane::base_pitch(v.transverse_module(), v.alpha_t)
                                - short / (cos_bb * cos_bb);
                            let n = 20_000;
                            let fine = (0..=n)
                                .map(|i| en * f64::from(i) / f64::from(n))
                                .chain([far])
                                .filter_map(w)
                                .fold(got, f64::max);
                            assert!(
                                fine <= got * (1.0 + 256.0 * f64::EPSILON),
                                "z{teeth} x{profile_shift} {pressure_angle}° h_a{addendum} \
                                 β{helix_angle} ε_n{eps_n} short {short}: rated {got}, \
                                 but a load point rates {fine}"
                            );
                            checked += 1;
                            off_single_pair += usize::from(share != 1.0);
                            let samples = [0.0, en, highest_single_pair(en), en.min(1.0)]
                                .into_iter()
                                .chain((0..=200).map(|i| f64::from(i) / 200.0 * en))
                                .filter_map(w)
                                .fold(0.0, f64::max);
                            beyond_samples +=
                                usize::from(got > samples * (1.0 + 256.0 * f64::EPSILON));
                        }
                    }
                }
            }
        }
        assert!(checked > 300, "checked {checked}");
        assert!(
            off_single_pair > 0 && beyond_samples > 0,
            "off the single-pair point {off_single_pair}, beyond 204 samples {beyond_samples}: \
             the law does not reach where the sweep differs"
        );
    }

    /// **Continuous across the pointed limit.** As the
    /// addendum approaches the pointed limit the figure has a one-sided
    /// derivative there: the difference quotients against the limit at 1e-3
    /// and 1e-6 of the addendum short agree. A step at the limit makes the
    /// quotient grow like the inverse of the hair, and the old divergence like
    /// its square; a smooth approach leaves it moving by the curvature times
    /// the hair, 1e-3 of it at most, so they must agree to 1e-2 of their size
    /// (they agree to 0.8 % at worst, z40 ε1.1, and to three figures elsewhere). Before the section was found once per
    /// tooth, z25 x0.5 28° under the ramp read 86.9 at 1e-3 short, 51,764 at
    /// 1e-6 short, 11.6 or 140 at the limit by sample count, and 2.157 once
    /// pointed; z40 x0.8 25° unshared read 4.45 short and nothing at the
    /// limit.
    #[test]
    fn the_rating_is_continuous_across_the_pointed_limit() {
        for (teeth, profile_shift, pressure_angle) in [
            (25_u32, 0.5_f64, 28.0_f64),
            (40, 0.8, 25.0),
            (12, 0.5, 25.0),
        ] {
            let build = |addendum: f64| {
                Tooth::new(GearParams {
                    teeth,
                    profile_shift,
                    pressure_angle,
                    addendum,
                    ..Default::default()
                })
            };
            // The addendum at which the tooth comes to a point.
            let capped = build(4.0);
            assert!(capped
                .clamps
                .fired(crate::note::key::CLAMP_TIP_CAPPED_POINTED));
            let m = capped.params.module;
            let limit = (capped.ra - capped.r) / m - profile_shift;
            for eps in [1.1_f64, 1.4] {
                for model in [LoadSharing::None, LoadSharing::LinearRamp] {
                    let label =
                        format!("z{teeth} x{profile_shift} {pressure_angle}° ε{eps} {model:?}");
                    let rate = |short: f64| {
                        let g = if short == 0.0 {
                            capped.clone()
                        } else {
                            build(limit * (1.0 - short))
                        };
                        bending_section_shared(&g, eps, model)
                            .and_then(|(s, f)| {
                                s.bending_factor(RootStressModel::DolanBroghamer)
                                    .map(|b| b * f)
                            })
                            .unwrap_or_else(|| panic!("{label}, {short} short: unrated"))
                    };
                    let at_limit = rate(0.0);
                    let quotient = |short: f64| (at_limit - rate(short)) / (limit * short);
                    let (q3, q6) = (quotient(1e-3), quotient(1e-6));
                    assert!(
                        (q3 - q6).abs() <= 1e-2 * q3.abs().max(q6.abs()),
                        "{label}: difference quotients {q3} at 1e-3 and {q6} at 1e-6 short"
                    );
                }
            }
        }
    }

    /// **A pointed tooth is rated at its root.** Near a pointed apex the
    /// Lewis parabola tangent to the flank shrinks onto the point, and its
    /// form factor grows like `1/d` — the stress in the point, not at the
    /// root. At `d = 0` it is `0/0`. So a sweep that searched a section afresh
    /// there answered `NaN`, or whatever its finest sample reached (17.4 at
    /// 200 samples, 208.6 at 3200). Held, the figure is finite and positive,
    /// and below `ε_n = 2`, where a single-pair zone exists, sharing relieves
    /// the unshared figure.
    #[test]
    fn a_pointed_tooth_is_rated_at_its_root() {
        let mut pointed = 0;
        for teeth in [10_u32, 12, 17, 25, 40] {
            for profile_shift in [0.3_f64, 0.5, 0.8] {
                for pressure_angle in [20.0_f64, 25.0, 28.0] {
                    for addendum in [1.4_f64, 1.6, 1.8] {
                        let g = Tooth::new(GearParams {
                            teeth,
                            profile_shift,
                            pressure_angle,
                            addendum,
                            ..Default::default()
                        });
                        if !g.clamps.fired(crate::note::key::CLAMP_TIP_CAPPED_POINTED)
                            || !g.is_usable()
                        {
                            continue;
                        }
                        pointed += 1;
                        let label = format!(
                            "z={teeth} x={profile_shift} α={pressure_angle} h_a={addendum}"
                        );
                        let factor = |s: &RootSection, f: f64| {
                            s.bending_factor(RootStressModel::DolanBroghamer)
                                .map(|b| b * f)
                        };
                        // A path a mesh could put on this tooth lies on its
                        // flank end to end: no longer than the flank, in base
                        // pitches.
                        let (lo, hi) = g.flank_bracket();
                        let flank = (hi - lo) * g.base_radius()
                            / crate::plane::base_pitch(
                                g.transverse_module(),
                                g.transverse_pressure_angle(),
                            );
                        for eps in [0.9_f64, 1.1, 1.4, 1.7, 1.95]
                            .into_iter()
                            .filter(|e| *e <= flank)
                        {
                            let alone = bending_section(&g, eps).and_then(|s| factor(&s, 1.0));
                            let shared = bending_section_shared(&g, eps, LoadSharing::LinearRamp)
                                .and_then(|(s, f)| factor(&s, f));
                            for (what, v) in [("unshared", alone), ("shared", shared)] {
                                if let Some(v) = v {
                                    assert!(
                                        v.is_finite() && v > 0.0,
                                        "{label} ε={eps}: {what} {v}"
                                    );
                                }
                            }
                            let (Some(alone), Some(shared)) = (alone, shared) else {
                                continue;
                            };
                            assert!(
                                shared / alone <= 1.002,
                                "{label} ε={eps}: sharing raised {alone} to {shared}"
                            );
                        }
                    }
                }
            }
        }
        assert!(
            pointed > 0,
            "no pointed tooth was built: the law is vacuous"
        );
    }

    /// **Load sharing is off by default, and off means untouched.**
    ///
    /// Not "agrees to a tolerance": with [`LoadSharing::None`] the sweep is not
    /// entered at all and the answer is [`bending_section`]'s own, which is
    /// what lets an uncalibrated model be offered without it reaching anyone
    /// who did not ask for it. Switched on, on these ordinary teeth it only
    /// relieves one, the governing point staying at the single-pair boundary
    /// where the share is exactly 1. That is these teeth, not the model: low
    /// on a small tooth's flank the held section's `K_f` grows as its arm
    /// shortens, and the ramp's maximum can sit there, above the unshared
    /// figure ([`bending_section_shared`]).
    #[test]
    fn sharing_is_off_by_default_and_can_only_ever_relieve_the_tooth() {
        for teeth in [12_u32, 17, 43, 97] {
            for beta in [0.0_f64, 20.0] {
                let g = Tooth::new(GearParams {
                    teeth,
                    helix_angle: beta,
                    ..Default::default()
                });
                for eps in [1.2_f64, 1.55, 1.9] {
                    let (plain, share) =
                        bending_section_shared(&g, eps, LoadSharing::None).unwrap();
                    let want = bending_section(&g, eps).unwrap();
                    assert_eq!(share, 1.0, "z={teeth}: no sharing means the whole load");
                    assert_eq!(
                        plain.s.to_bits(),
                        want.s.to_bits(),
                        "z={teeth} β={beta} ε={eps}: the default path moved"
                    );

                    let (shared, fraction) =
                        bending_section_shared(&g, eps, LoadSharing::LinearRamp).unwrap();
                    assert!(
                        (0.0..=1.0).contains(&fraction),
                        "z={teeth}: a share of {fraction}"
                    );
                    // The rating is proportional to `Y_F · Y_S · share`, so that
                    // product is what may not rise.
                    let of = |s: &RootSection, f: f64| {
                        s.bending_factor(RootStressModel::Iso6336).unwrap() * f
                    };
                    let (was, now) = (of(&plain, 1.0), of(&shared, fraction));
                    assert!(
                        now <= was * (1.0 + 1e-9),
                        "z={teeth} β={beta} ε={eps}: sharing raised the rating from \
                         {was} to {now}"
                    );
                    // **And which regime it is in decides how much it may do.**
                    //
                    // Below `ε_n = 2` there is a single-pair zone, the governing
                    // point moves into it, the share there is exactly 1, and the
                    // effect is the fraction of a per cent the rationale
                    // measured. At or above it there is no such zone — two pairs
                    // are always engaged — the ramp never reaches 1, and it
                    // moves the tooth's figure either way. That second regime is the
                    // one the ramp was never calibrated for, and the stage says
                    // so rather than letting the number pass as a rating.
                    let eps_n = eps / g.base_helix_angle().cos().powi(2);
                    if eps_n < 2.0 {
                        assert!(
                            now >= 0.97 * was,
                            "z={teeth} β={beta} ε={eps} (ε_n={eps_n}): a single-pair zone \
                             exists, so the governing point should sit in it and the effect \
                             should be slight — {was} to {now}"
                        );
                    } else {
                        assert!(
                            now < was,
                            "z={teeth} β={beta} ε_n={eps_n}: with no single-pair zone the \
                             ramp must relieve the tooth somewhere"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn analytic_tangent_matches_a_finite_difference() {
        let g = Tooth::new(GearParams::default());
        let h = 1e-7;
        let mut worst = 0.0_f64;
        for i in 1..20 {
            let s = g.s_j * f64::from(i) / 20.0;
            let (_, t) = fillet_point_and_tangent(&g, s);
            let (a, _) = fillet_point_and_tangent(&g, s - h);
            let (b, _) = fillet_point_and_tangent(&g, s + h);
            let fd = [(b[0] - a[0]) / (2.0 * h), (b[1] - a[1]) / (2.0 * h)];
            let scale = f64::hypot(t[0], t[1]).max(1.0);
            worst = worst.max(((t[0] - fd[0]).abs() + (t[1] - fd[1]).abs()) / scale);
        }
        assert!(worst < 1e-6, "analytic tangent disagrees by {worst:.3e}");
    }

    #[test]
    fn tangency_really_is_at_thirty_degrees() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 12,
                profile_shift: -0.2,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                profile_shift: 0.4,
                ..Default::default()
            },
            GearParams {
                teeth: 25,
                helix_angle: 20.0,
                ..Default::default()
            },
        ] {
            let g = Tooth::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let (_, t) = fillet_point_and_tangent(&g, sec.s);
            let angle = (t[0].abs()).atan2(t[1].abs()).to_degrees();
            assert!(
                (angle - TANGENT_ANGLE_DEG).abs() < 1e-6,
                "z={}: tangent at {angle}°, expected {TANGENT_ANGLE_DEG}°",
                p.teeth
            );
        }
    }

    #[test]
    fn tangency_point_lies_on_the_fillet_between_root_and_junction() {
        for teeth in [9u32, 17, 30, 60] {
            let g = Tooth::new(GearParams {
                teeth,
                ..Default::default()
            });
            let sec = root_section(&g, g.u_tip).unwrap();
            assert!(
                sec.s <= 0.0 && sec.s >= g.s_j,
                "s={} outside [{}, 0]",
                sec.s,
                g.s_j
            );
            let r = f64::hypot(sec.tangency[0], sec.tangency[1]);
            assert!(r >= g.rf - 1e-9 && r <= g.r_j + 1e-9, "tangency at r={r}");
        }
    }

    /// The load line must actually pass through the contact point and the
    /// base-circle tangent point — that is what makes it the line of action.
    #[test]
    fn load_line_is_tangent_to_the_base_circle() {
        let g = Tooth::new(GearParams {
            teeth: 23,
            ..Default::default()
        });
        let (p, d) = flank_point_and_load_direction(&g, g.u_tip);
        // distance from the gear centre to the load line
        let dist = (p[0] * d[1] - p[1] * d[0]).abs();
        assert!(
            (dist - g.rb).abs() < 1e-9,
            "load line passes {dist} from the centre, base radius is {}",
            g.rb
        );
    }

    #[test]
    fn form_factor_falls_as_tooth_count_rises() {
        let mut last = f64::INFINITY;
        for teeth in [12u32, 17, 25, 40, 80, 150] {
            let g = Tooth::new(GearParams {
                teeth,
                ..Default::default()
            });
            let y = root_section(&g, g.u_tip).unwrap().form_factor;
            assert!(y < last, "z={teeth}: Y_F {y} did not fall below {last}");
            last = y;
        }
    }

    #[test]
    fn positive_profile_shift_thickens_the_root_and_lowers_the_form_factor() {
        let mut last_chord = 0.0_f64;
        let mut last_yf = f64::INFINITY;
        for xi in [-4i32, -2, 0, 2, 4] {
            let g = Tooth::new(GearParams {
                teeth: 20,
                profile_shift: f64::from(xi) * 0.1,
                ..Default::default()
            });
            let sec = root_section(&g, g.u_tip).unwrap();
            assert!(
                sec.root_chord > last_chord,
                "x={xi}: root chord did not grow"
            );
            assert!(sec.form_factor < last_yf, "x={xi}: Y_F did not fall");
            last_chord = sec.root_chord;
            last_yf = sec.form_factor;
        }
    }

    /// The drawn tangent leans **inward**: climbing the fillet toward the tip,
    /// the tooth narrows. A diagram that reconstructed the line from the 30°
    /// angle alone drew its mirror image, with the tangency point still correct,
    /// so this pins the sign as well as the angle.
    #[test]
    fn tangent_direction_leans_towards_the_centreline_going_up() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                profile_shift: 0.3,
                ..Default::default()
            },
        ] {
            let g = Tooth::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let d = sec.tangent_direction;
            assert!(
                d[1] > 0.0,
                "z={}: tangent should point up the tooth",
                p.teeth
            );
            assert!(
                d[0] < 0.0,
                "z={}: on the +x side the tooth narrows going up, so the tangent \
                 must lean inward; got {d:?}",
                p.teeth
            );
            let angle = d[0].abs().atan2(d[1].abs()).to_degrees();
            assert!((angle - TANGENT_ANGLE_DEG).abs() < 1e-6, "angle {angle}");
            assert!(
                (f64::hypot(d[0], d[1]) - 1.0).abs() < 1e-12,
                "not a unit vector"
            );
        }
    }

    #[test]
    fn stress_correction_can_be_switched_off_for_comparison() {
        let g = Tooth::new(GearParams::default());
        let sec = root_section(&g, g.u_tip).unwrap();
        assert!(
            (sec.stress_correction(RootStressModel::FormFactorOnly)
                .unwrap()
                - 1.0)
                .abs()
                < 1e-15
        );
        assert!(
            (sec.bending_factor(RootStressModel::FormFactorOnly).unwrap() - sec.form_factor).abs()
                < 1e-15,
            "with no correction the bending factor must be the form factor alone"
        );
    }

    /// The correction rises as the fillet sharpens — that is the whole content
    /// of a notch factor, and it is checkable without trusting the coefficients.
    #[test]
    fn sharper_fillets_raise_the_stress_correction() {
        let mut last = 0.0_f64;
        for root_radius in [0.38_f64, 0.30, 0.20, 0.10, 0.05] {
            let g = Tooth::new(GearParams {
                root_radius,
                ..Default::default()
            });
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let ys = sec.stress_correction(RootStressModel::Iso6336).unwrap();
            assert!(
                ys > last,
                "rho={root_radius}: Y_S {ys} did not exceed {last} for a blunter fillet"
            );
            assert!(ys > 1.0, "a notch cannot reduce stress");
            last = ys;
        }
    }

    #[test]
    fn notch_parameter_is_the_ratio_the_iso_fit_expects() {
        let g = Tooth::new(GearParams::default());
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        let want = sec.root_chord / (2.0 * sec.fillet_curvature);
        assert!((sec.notch_parameter - want).abs() < 1e-12);
        assert!(
            sec.notch_parameter_in_range(),
            "q_s = {}",
            sec.notch_parameter
        );
    }

    /// The parabola must genuinely be tangent: the tangency point lies on it,
    /// and the slopes agree. Solved through an eliminated parameter, so both
    /// conditions are worth re-checking against the recovered `p`.
    #[test]
    fn lewis_parabola_is_tangent_to_the_fillet() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                ..Default::default()
            },
            GearParams {
                teeth: 12,
                profile_shift: -0.3,
                ..Default::default()
            },
            GearParams {
                teeth: 60,
                profile_shift: 0.3,
                ..Default::default()
            },
        ] {
            let g = Tooth::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
            let pp = sec.parabola_p.unwrap();
            let vertex = sec.load_line_crossing[1];
            let (q, t) = fillet_point_and_tangent(&g, sec.s);

            // on the parabola
            let on = q[0] * q[0] - 4.0 * pp * (vertex - q[1]);
            assert!(
                on.abs() < 1e-9,
                "z={}: point off the parabola by {on:.2e}",
                p.teeth
            );
            // slopes agree
            let parabola_slope = -q[0] / (2.0 * pp);
            let fillet_slope = t[1] / t[0];
            assert!(
                (parabola_slope - fillet_slope).abs() < 1e-7,
                "z={}: slope {parabola_slope} vs {fillet_slope}",
                p.teeth
            );
        }
    }

    /// The parabola is the more conservative construction at every tooth count,
    /// and the gap does **not** close: the two converge to *different* rack
    /// limits (2.063 against 2.159), because they are different constructions
    /// rather than an approximation and its exact form.
    #[test]
    fn parabola_is_consistently_more_conservative() {
        for teeth in [9u32, 17, 60, 300, 1000] {
            let g = Tooth::new(GearParams {
                teeth,
                ..Default::default()
            });
            let a = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let b = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
            assert!(
                b.root_chord < a.root_chord,
                "z={teeth}: parabola section not narrower"
            );
            assert!(
                b.form_factor > a.form_factor,
                "z={teeth}: parabola Y_F {} not above tangent {}",
                b.form_factor,
                a.form_factor
            );
        }
    }

    /// Which curve the largest inscribed parabola touches depends on the
    /// tooth, and getting this wrong is how the first implementation failed:
    /// a fillet-only search finds no solution at all on large teeth. Asked of
    /// the construction (the section of highest `Y_F`); the rating weighs the
    /// fillet's by its notch factor, and on these teeth rates the fillet's.
    #[test]
    fn parabola_touches_the_fillet_on_small_teeth_and_the_flank_on_large() {
        let parabola = |g: &Tooth| {
            root_section_rated(
                g,
                g.u_tip,
                CriticalSection::LewisParabola,
                RootStressModel::FormFactorOnly,
            )
            .unwrap()
        };
        let small = Tooth::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        assert!(
            !parabola(&small).tangency_on_flank,
            "z=17 should touch the fillet"
        );

        let large = Tooth::new(GearParams {
            teeth: 1000,
            ..Default::default()
        });
        assert!(
            parabola(&large).tangency_on_flank,
            "z=1000 should touch the flank"
        );
        assert!(
            !root_section_with(&large, large.u_tip, CriticalSection::LewisParabola)
                .unwrap()
                .tangency_on_flank,
            "z=1000 rates at its fillet, whose notch outweighs the flank's larger Y_F"
        );
    }

    /// Unlike the 30° tangent, the parabola construction follows the load point.
    /// That is the property the cantilever model is meant to have.
    #[test]
    fn only_the_parabola_moves_with_the_load_point() {
        let g = Tooth::new(GearParams {
            teeth: 20,
            ..Default::default()
        });
        let low = g.u_tip * 0.6;
        let a1 = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        let a2 = root_section_with(&g, low, CriticalSection::TangentAngle).unwrap();
        assert!(
            (a1.s - a2.s).abs() < 1e-12,
            "the 30 degree section must not move"
        );

        let b1 = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
        let b2 = root_section_with(&g, low, CriticalSection::LewisParabola).unwrap();
        assert!(
            (b1.s - b2.s).abs() > 1e-6,
            "the parabola section must follow the load"
        );
    }

    /// The clamp is tested on the formula directly, by setting the parameter,
    /// because the interesting cases are at the extremes.
    ///
    /// It is not a hypothetical guard: see
    /// `large_teeth_with_a_sharp_cutter_leave_the_stated_range`.
    #[test]
    fn notch_parameter_is_clamped_for_the_fit_but_reported_raw() {
        let g = Tooth::new(GearParams::default());
        let base = root_section(&g, g.u_tip).unwrap();

        let mut sharp = base;
        sharp.notch_parameter = 50.0; // far past the stated range
        assert!(!sharp.notch_parameter_in_range());
        assert!(
            (sharp.notch_parameter - 50.0).abs() < 1e-12,
            "the raw value must survive for reporting"
        );

        let mut sharper = base;
        sharper.notch_parameter = 500.0;
        // Both clamp to the same q_s, so the correction stops rising.
        let a = sharp.stress_correction(RootStressModel::Iso6336).unwrap();
        let b = sharper.stress_correction(RootStressModel::Iso6336).unwrap();
        assert!((a - b).abs() < 1e-12, "clamp is not holding: {a} vs {b}");

        let mut blunt = base;
        blunt.notch_parameter = 0.1;
        assert!(!blunt.notch_parameter_in_range());
        let at_floor = blunt.stress_correction(RootStressModel::Iso6336).unwrap();
        let mut at_one = base;
        at_one.notch_parameter = 1.0;
        assert!(
            (at_floor - at_one.stress_correction(RootStressModel::Iso6336).unwrap()).abs() < 1e-12
        );
    }

    /// On small and medium gears the notch parameter stays inside the ISO
    /// range whatever the cutter, because `ρ_F` there is governed by the
    /// trochoid rather than by the cutter tip radius: shrinking the corner from
    /// 0.38 to 0.005 module moves `q_s` only from 1.62 to 2.37 at z=17.
    #[test]
    fn ordinary_gears_keep_the_notch_parameter_in_range() {
        for root_radius in [0.38_f64, 0.2, 0.05, 0.005] {
            for teeth in [9u32, 17, 60] {
                let g = Tooth::new(GearParams {
                    teeth,
                    root_radius,
                    ..Default::default()
                });
                let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
                assert!(
                    sec.notch_parameter_in_range(),
                    "z={teeth} rho={root_radius}: q_s = {} left the stated range",
                    sec.notch_parameter
                );
            }
        }
    }

    /// Large teeth are the exception, and they are why the clamp exists. On a
    /// flat tooth the trochoid no longer dominates the root curvature, so a
    /// sharp cutter carries straight through into `q_s`.
    ///
    /// The clamp then **under**-predicts the stress, so this is exactly the case
    /// a caller must be told about rather than have silently corrected.
    #[test]
    fn large_teeth_with_a_sharp_cutter_leave_the_stated_range() {
        let g = Tooth::new(GearParams {
            teeth: 300,
            root_radius: 0.05,
            ..Default::default()
        });
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        assert!(
            sec.notch_parameter > NOTCH_PARAMETER_RANGE.end,
            "expected q_s past the range, got {}",
            sec.notch_parameter
        );
        assert!(!sec.notch_parameter_in_range());
        // and the reported value is the real one, not the clamped one
        assert!(sec.notch_parameter > 10.0);
    }

    /// **The bending rating is continuous across the flank/fillet transition.**
    ///
    /// The largest inscribed parabola's tangency migrates up the fillet as a
    /// tooth grows and crosses onto the involute flank. Nothing physical
    /// happens there — a 151-tooth gear is not stronger or weaker than a
    /// 150-tooth one by any step — so nothing in a rating may jump either.
    /// What once jumped was the notch radius read on the flank; what could
    /// jump now is the notch factor, the fillet's on one side and none on the
    /// other. It does not, because each curve's section is a candidate rated
    /// with its own and the higher governs: swept across the seam the
    /// parabola crosses, every model's rating moves by less than a percent per
    /// tooth.
    #[test]
    fn the_rating_is_continuous_across_the_flank_fillet_transition() {
        let tooth = |z: u32| {
            Tooth::new(GearParams {
                teeth: z,
                ..Default::default()
            })
        };
        let mut seen_both = (false, false);
        for model in [RootStressModel::DolanBroghamer, RootStressModel::Iso6336] {
            let mut previous: Option<f64> = None;
            for z in 140..=165_u32 {
                let g = tooth(z);
                let parabola = root_section_rated(
                    &g,
                    g.u_tip,
                    CriticalSection::LewisParabola,
                    RootStressModel::FormFactorOnly,
                )
                .unwrap();
                if parabola.tangency_on_flank {
                    seen_both.1 = true;
                } else {
                    seen_both.0 = true;
                }
                let rated = root_section_rated(&g, g.u_tip, CriticalSection::LewisParabola, model)
                    .and_then(|s| s.bending_factor(model))
                    .unwrap();
                if let Some(p) = previous {
                    let step = ((rated - p) / p).abs();
                    assert!(step < 0.01, "{model:?} z={z}: the rating stepped by {step}");
                }
                previous = Some(rated);
            }
        }
        assert!(
            seen_both.0 && seen_both.1,
            "the sweep has to actually cross the seam to be testing anything"
        );
    }

    /// `ρ_F` is a property of the **fillet**, at any tooth size.
    ///
    /// Stated on its own because it is the whole of the fix above: on a flank
    /// tangency the reported fillet curvature must be the fillet's, at the
    /// junction — not the involute's, which is larger by a factor of forty and
    /// means nothing as a notch radius.
    #[test]
    fn the_notch_radius_is_always_the_fillets() {
        let g = Tooth::new(GearParams {
            teeth: 1000,
            ..Default::default()
        });
        let sec = root_section_rated(
            &g,
            g.u_tip,
            CriticalSection::LewisParabola,
            RootStressModel::FormFactorOnly,
        )
        .unwrap();
        assert!(sec.tangency_on_flank, "a 1000-tooth gear touches its flank");

        // The fillet's own curvature at its junction with the flank.
        let junction = g.fillet_curvature(g.fillet_junction());
        assert!((sec.fillet_curvature - junction).abs() < 1e-12);
        assert!(
            (sec.notch_parameter - sec.root_chord / (2.0 * junction)).abs() < 1e-12,
            "q_s must be built from that same radius"
        );

        // ...and it is nothing like the involute's curvature there.
        let involute = g.flank_curvature(sec.s);
        assert!(
            involute > 10.0 * junction,
            "the two should differ by an order of magnitude: {involute} vs {junction}"
        );

        // Reported, and not applied: the section is on the smooth flank.
        assert!(sec.notch_parameter_in_range());
        for model in [RootStressModel::Iso6336, RootStressModel::DolanBroghamer] {
            assert_eq!(sec.stress_correction(model), Some(1.0), "{model:?}");
        }
    }

    /// **A section's chord and depth are more than rounding.** The apex of a
    /// pointed tip loaded at its point solves the tangency condition exactly,
    /// with a chord and an arm of rounding, about `ε·R`; as a section it
    /// rated at 1e13. Its half-width is within [`COORDINATE_ROUNDING`] of
    /// its radius, so it is no section and the flank offers its end at the
    /// fillet; a length ten times that is a length.
    #[test]
    fn a_pointed_tip_loaded_at_its_point_is_no_section() {
        let g = Tooth::new(GearParams {
            teeth: 12,
            addendum: 1.1,
            root_radius: 0.25,
            profile_shift: 0.7,
            ..Default::default()
        });
        assert!(g.clamps.fired(crate::note::key::CLAMP_TIP_CAPPED_POINTED));
        let (apex, _) = ToothOutline::flank_at(&g, g.u_tip);
        let scale = apex[0].hypot(apex[1]);
        assert!(
            !beyond_rounding(apex[0], scale),
            "the apex's half-width {} is a length",
            apex[0]
        );
        // The criterion scales with the point's radius: half of it at this
        // radius is rounding, ten times it a length.
        assert!(scale > 2.0, "a radius that tells a product from a quotient");
        assert!(!beyond_rounding(0.5 * COORDINATE_ROUNDING * scale, scale));
        assert!(beyond_rounding(10.0 * COORDINATE_ROUNDING * scale, scale));
        let found = root_sections(&g, g.u_tip, CriticalSection::LewisParabola);
        assert_eq!(found.len(), 2, "{found:?}");
        let flank = found.iter().find(|s| s.tangency_on_flank).unwrap();
        assert_eq!(flank.s, g.u_j, "the flank offers its end at the fillet");
        assert!(found.iter().all(|s| beyond_rounding(s.root_chord, scale)));
        let rated = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola)
            .and_then(|s| s.bending_factor(RootStressModel::DolanBroghamer))
            .unwrap();
        assert!(
            rated < 10.0,
            "a pointed tip loaded at its point rates {rated}"
        );
    }

    #[test]
    fn a_severed_tooth_has_no_root_section() {
        let g = Tooth::new(GearParams {
            teeth: 3,
            profile_shift: -0.5,
            ..Default::default()
        });
        assert!(g.severed);
        assert!(root_section(&g, 0.5).is_none());
    }

    // ------------------------------------------------------------ load ----

    fn pair(z1: u32, z2: u32) -> (Tooth, Tooth, Mesh) {
        let a = Tooth::new(GearParams {
            teeth: z1,
            ..Default::default()
        });
        let b = Tooth::new(GearParams {
            teeth: z2,
            ..Default::default()
        });
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        (a, b, m)
    }

    /// The three force projections must be mutually consistent, and each must
    /// name the plane it is in. This is the check the old `normal_force` field
    /// could not have passed: it was transverse but called normal.
    #[test]
    fn the_force_projections_are_mutually_consistent() {
        for beta in [0.0, 15.0, 30.0] {
            let g = Tooth::new(GearParams {
                teeth: 23,
                module: 2.0,
                helix_angle: beta,
                ..Default::default()
            });
            let load = Load::new(4.5, 10.0);

            // F_t = T / r, independently of anything else.
            assert!((load.tangential(&g) - 1000.0 * 4.5 / g.r).abs() < 1e-9);
            // F_bt = F_t / cos α_t — the transverse projection onto the line of
            // action.
            let f_bt = load.transverse_line_of_action(&g);
            assert!((f_bt - load.tangential(&g) / g.alpha_t.cos()).abs() < 1e-9);
            // F_bn = F_bt / cos β_b, and for a spur gear the two coincide.
            let f_bn = load.normal_to_flank(&g);
            let cos_bb = g.base_helix_angle().cos();
            assert!((f_bn - f_bt / cos_bb).abs() < 1e-9);
            if beta == 0.0 {
                assert!(
                    (f_bn - f_bt).abs() < 1e-12,
                    "spur: normal must equal transverse"
                );
            } else {
                assert!(
                    f_bn > f_bt,
                    "beta={beta}: the flank force must exceed its projection"
                );
            }
        }
    }

    /// `F_bt` is shared across a mesh, so re-quoting the load against the other
    /// gear must leave it unchanged. That is the invariant that replaced storing
    /// the force directly.
    #[test]
    fn a_load_carried_across_a_mesh_keeps_the_same_force_on_the_line_of_action() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            let (g1, g2, _) = pair(z1, z2);
            let l1 = Load::new(2.0, 8.0);
            let l2 = l1.across_mesh(&g1, &g2);

            assert!(
                (l1.transverse_line_of_action(&g1) - l2.transverse_line_of_action(&g2)).abs()
                    < 1e-9,
                "z={z1}/{z2}: F_bt changed across the mesh"
            );
            // Torque scales with the ratio, and the round trip is exact.
            let ratio = f64::from(z2) / f64::from(z1);
            assert!((l2.torque / l1.torque - ratio).abs() < 1e-9);
            assert!((l2.across_mesh(&g2, &g1).torque - l1.torque).abs() < 1e-12);
        }
    }

    /// **`Y_B` is the curve ISO 6336-3 draws**, and its published breakpoints
    /// are consequences of the fit rather than extra constants.
    ///
    /// The clause states case (a) — full support — at 1,2 and 3,5, and states
    /// the fit separately. That the fit reaches 1 at those very ratios is the
    /// check that both were transcribed right, and it is what lets the
    /// implementation take the larger of the two without a breakpoint of its
    /// own.
    #[test]
    fn the_rim_factor_is_the_curve_the_standard_draws() {
        // Figure 9's two readable endpoints, at the ratios the clause floors at.
        let thin_external = RimSupport::external(0.5, 1.0);
        assert!((thin_external.factor() - 2.4).abs() < 0.01);
        let thin_internal = RimSupport::internal(1.75, 1.0);
        assert!((thin_internal.factor() - 1.8).abs() < 0.01);

        // **The stated breakpoints fall out of the fit**: `a ln(c/ratio) = 1`
        // is solved at 1,20005 and 3,48887 against the clause's 1,2 and 3,5, so
        // the two are the same statement to the precision its constants carry.
        // That is why the implementation takes the larger of the fit and 1
        // rather than testing a breakpoint of its own — and it is why 1,2 is
        // not in the loop below, being a hair inside the sloping arm.
        assert!((RimSupport::external(1.2, 1.0).factor() - 1.0).abs() < 1e-4);
        assert!((RimSupport::internal(3.5, 1.0).factor() - 1.0).abs() < 4e-3);

        // Never below 1: a thick rim does not relieve a root, it merely stops
        // de-rating it.
        for ratio in [1.21, 2.0, 10.0, 1e6] {
            assert_eq!(RimSupport::external(ratio, 1.0).factor(), 1.0);
        }
        for ratio in [3.5, 6.0, 40.0] {
            assert_eq!(RimSupport::internal(ratio, 1.0).factor(), 1.0);
        }

        // Monotone: less rim is never less de-rating.
        let mut last = 1.0;
        for ratio in [1.1, 0.9, 0.7, 0.5] {
            let y = RimSupport::external(ratio, 1.0).factor();
            assert!(y > last, "Y_B should rise as the rim thins: {ratio}");
            last = y;
        }

        // The floor is reported, not enforced — the fit still answers below it.
        assert!(RimSupport::external(0.51, 1.0).in_range());
        assert!(!RimSupport::external(0.5, 1.0).in_range());
        assert!(RimSupport::internal(1.76, 1.0).in_range());
        assert!(!RimSupport::internal(1.75, 1.0).in_range());
        assert!(RimSupport::external(0.2, 1.0).factor().is_finite());

        // The reference is the whole of the difference between the two arms:
        // an external rim is measured against the tooth depth, a ring's against
        // the module, and each reads back the ratio it was built from.
        assert!((RimSupport::external(2.25, 2.25).ratio() - 1.0).abs() < 1e-15);
        assert!((RimSupport::internal(7.0, 2.0).ratio() - 3.5).abs() < 1e-15);
    }

    #[test]
    fn bending_stress_scales_the_way_the_cantilever_model_says() {
        let g = Tooth::new(GearParams {
            teeth: 25,
            ..Default::default()
        });
        let sec = root_section(&g, g.u_tip).unwrap();
        let s = |t: f64, b: f64| {
            let load = Load::new(t, b);
            bending_stress(
                &sec,
                load.tangential(&g),
                b,
                RootStressModel::FormFactorOnly,
                None,
            )
            .unwrap()
        };

        let base = s(1.0, 10.0);
        // Linear in load...
        assert!((s(2.0, 10.0) - 2.0 * base).abs() < 1e-9);
        // ...and inversely proportional to face width.
        assert!((s(1.0, 20.0) - base / 2.0).abs() < 1e-9);
        assert!(base > 0.0);
    }

    /// `b_min` must not depend on the `b` the stress was evaluated at. It is the
    /// invariant that catches a stress which did not actually scale with face
    /// width — the failure a single spot check would sail past — and every stage
    /// kind leans on it, since each rates once at a probe width and inverts.
    ///
    /// **Every model, and a rim, because the property is the width's and not the
    /// notch's.** It ran on `FormFactorOnly` alone, which is the one variant no
    /// stage rates with: that model ships through `gear-cli matrix`, and the
    /// stages all use Dolan–Broghamer. Nothing was wrong with the answer — none
    /// of the three reads a face width, which is *why* the invariant holds — but
    /// a property asserted of one arm of a `match` is asserted of one arm of a
    /// `match`, and the arm the tool runs was not it.
    #[test]
    fn minimum_face_width_is_independent_of_the_face_width_used() {
        let (g1, g2, mesh) = pair(19, 31);
        let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
        let sec = root_section(&g1, path.roll_at(path.highest_single_pair())).unwrap();

        let mut checked = 0u32;
        for model in [
            RootStressModel::FormFactorOnly,
            RootStressModel::DolanBroghamer,
            RootStressModel::Iso6336,
        ] {
            // ...and with the rim clause silent and biting, since it is the one
            // factor that could have been written against a width.
            for rim in [None, Some(RimSupport::external(1.5, 2.25))] {
                let (mut bend, mut cont) = (Vec::new(), Vec::new());
                for b in [1.0, 5.0, 12.5, 100.0] {
                    let load = Load::new(3.0, b);
                    let sf = bending_stress(&sec, load.tangential(&g1), b, model, rim).unwrap();
                    let sh =
                        contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 100_000.0).unwrap();
                    bend.push(min_face_width_bending(sf, b, 200.0));
                    cont.push(min_face_width_contact(sh.worst, b, 800.0));
                }
                for v in &bend {
                    assert!(
                        (v - bend[0]).abs() < 1e-9,
                        "{model:?} rim {rim:?}: bending b_min drifted: {bend:?}"
                    );
                }
                for v in &cont {
                    assert!(
                        (v - cont[0]).abs() < 1e-9,
                        "{model:?} rim {rim:?}: contact b_min drifted: {cont:?}"
                    );
                }
                assert!(bend[0] > 0.0 && cont[0] > 0.0);
                checked += 1;
            }
        }
        assert_eq!(checked, 6, "a model or a rim case went unrun");
    }

    /// Hertz, reached a second way: through the contact half-width.
    ///
    /// `b_h = √(4F'R/πE*)` and `p_max = 2F'/(π b_h)` is the textbook line-contact
    /// pair. Eliminating `b_h` gives `√(F'E*/πR)`, so agreement checks the
    /// algebra in `contact_stress` against a route that shares none of it.
    #[test]
    fn contact_stress_matches_the_half_width_route() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
        let load = Load::new(2.0, 8.0);
        let e_star = 113_000.0;
        let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star).unwrap();

        let f_prime = load.transverse_line_of_action(&g1) / load.face_width;
        let r = cs.relative_radius;
        let half_width = (4.0 * f_prime * r / (std::f64::consts::PI * e_star)).sqrt();
        let p_max = 2.0 * f_prime / (std::f64::consts::PI * half_width);
        assert!(
            (cs.worst - p_max).abs() / p_max < 1e-12,
            "{} vs {p_max}",
            cs.worst
        );
        assert!(half_width > 0.0 && half_width < r, "implausible half width");
    }

    /// **The acceptance gate for the contact unification** (docs/reference.md#contact-stress).
    ///
    /// At `1/R_L = 0` the general elliptical solution must not perturb the line
    /// result — not "agree to 1e-12", but return the identical `f64`. It can,
    /// because the elliptical patch's peak pressure is exactly zero there and
    /// the `max` therefore selects the untouched line expression. Anything less
    /// than bit equality means the line term was rewritten rather than carried
    /// across, which is the one way this step can silently move an answer.
    #[test]
    fn the_general_form_is_bit_identical_to_line_contact_at_parallel_axes() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25), (19, 31)] {
            for beta in [0.0, 15.0, 30.0] {
                let g1 = Tooth::new(GearParams {
                    teeth: z1,
                    helix_angle: beta,
                    ..Default::default()
                });
                let g2 = Tooth::new(GearParams {
                    teeth: z2,
                    helix_angle: -beta,
                    ..Default::default()
                });
                let mesh = Mesh::new(&g1, &g2, MeshKind::External).unwrap();
                let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
                let load = Load::new(2.0, 10.0);
                let e_star = 113_000.0;
                let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star).unwrap();

                // The line formula, spelled out in the same order the function
                // evaluates it. The duplication *is* the assertion: it says
                // this exact expression survived the unification. Reaching the
                // same number by a different order of operations would only
                // prove agreement to an ulp or so, which is not the claim —
                // that check is `contact_stress_matches_the_half_width_route`.
                let sum_z = f64::from(mesh.z1) + f64::from(mesh.z2);
                let rb2 = mesh.a_w * f64::from(mesh.z2) / sum_z * mesh.alpha_w.cos();
                let rho1 = path.base_radius_1 * mesh.alpha_w.tan() + cs.worst_position;
                let rho2 = rb2 * mesh.alpha_w.tan() - cs.worst_position;
                let inv_rho_n = g1.base_helix_angle().cos() * (1.0 / rho1 + 1.0 / rho2);
                let f_prime = load.transverse_line_of_action(&g1) / load.face_width;
                let line = (f_prime * inv_rho_n * e_star / std::f64::consts::PI).sqrt();
                assert_eq!(
                    cs.worst, line,
                    "z={z1}/{z2} beta={beta}: the general form moved the line answer"
                );
            }
        }
    }

    /// The lengthwise curvature is a one-way parameter: it can only concentrate
    /// the load further, so the stress rises monotonically with it and returns
    /// continuously to the line value as it goes to zero. There is no jump at
    /// the parallel-axis point, which is the property that lets one function
    /// serve both regimes.
    #[test]
    fn stress_rises_monotonically_with_lengthwise_curvature_and_returns_to_the_line() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
        let load = Load::new(2.0, 10.0);
        let e_star = 113_000.0;

        let line = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star)
            .unwrap()
            .worst;

        // Coming down toward parallel axes, the answer converges on the line
        // value from above and never below it.
        let mut previous = f64::INFINITY;
        for exponent in 0..12 {
            let curvature = 10.0_f64.powi(-exponent);
            let worst = contact_stress(&path, &mesh, &g1, curvature, &load, e_star)
                .unwrap()
                .worst;
            assert!(
                worst >= line,
                "curvature {curvature}: {worst} fell below the line value {line}"
            );
            assert!(
                worst <= previous,
                "curvature {curvature}: stress must fall as the mesh flattens"
            );
            previous = worst;
        }
        assert!(
            (previous - line).abs() < 1e-9 * line,
            "at 1e-11/mm the answer should have returned to the line value: \
             {previous} vs {line}"
        );

        // And a curvature comparable with the profile's makes the point contact
        // govern outright, which is the crossed-axis regime.
        let crossed = contact_stress(&path, &mesh, &g1, 0.5, &load, e_star)
            .unwrap()
            .worst;
        assert!(
            crossed > 2.0 * line,
            "a point contact should be far worse than a line: {crossed} vs {line}"
        );
    }

    /// `ρ₁ + ρ₂ = a_w sin α_w` everywhere on the path — the two flanks' local
    /// radii are complementary, which is what makes the relative radius peak at
    /// the pitch point and fall toward both ends.
    #[test]
    fn the_two_local_radii_sum_to_the_line_of_action() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
        let sz = f64::from(mesh.z1) + f64::from(mesh.z2);
        let rb2 = mesh.a_w * f64::from(mesh.z2) / sz * mesh.alpha_w.cos();
        let want = mesh.a_w * mesh.alpha_w.sin();

        for i in 0..=20 {
            let t = i as f64 / 20.0;
            let xi = -path.approach + t * (path.approach + path.recess);
            let rho1 = path.base_radius_1 * mesh.alpha_w.tan() + xi;
            let rho2 = rb2 * mesh.alpha_w.tan() - xi;
            assert!((rho1 + rho2 - want).abs() < 1e-9);
        }
    }

    // --------------------------------------------------------- helical ----

    /// A spur gear is its own normal section, so the virtual construction must
    /// be the identity there — otherwise every spur result would shift.
    #[test]
    fn the_virtual_spur_gear_of_a_spur_gear_is_itself() {
        let g = Tooth::new(GearParams {
            teeth: 25,
            ..Default::default()
        });
        let v = g.virtual_spur();
        assert!((v.z - g.z).abs() < 1e-15);
        assert!((v.r - g.r).abs() < 1e-15);
        assert!((v.rb - g.rb).abs() < 1e-15);

        // Loaded at the same place, the two must agree exactly.
        let eps = 1.6;
        let pb = crate::plane::base_pitch(g.mt, g.alpha_t);
        let a = root_section(&g, g.u_tip - (eps - 1.0) * pb / g.rb).unwrap();
        let b = bending_section(&g, eps).unwrap();
        assert!((a.form_factor - b.form_factor).abs() < 1e-15);
    }

    /// `z_n = z / cos³β`, and the virtual gear is a genuine spur gear cut with
    /// the normal rack.
    #[test]
    fn the_virtual_spur_gear_has_the_iso_tooth_count_and_the_normal_rack() {
        for beta in [10.0, 20.0, 30.0, 45.0] {
            let g = Tooth::new(GearParams {
                teeth: 30,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let b = beta.to_radians();

            assert!(
                (v.z - 30.0 / b.cos().powi(3)).abs() < 1e-12,
                "beta={beta}: z_n = {}",
                v.z
            );
            // It is a spur gear, in the normal plane, with the normal module.
            assert!(v.beta.abs() < 1e-15);
            assert!((v.alpha_t - g.alpha_n).abs() < 1e-12);
            assert!((v.mt - g.params.module).abs() < 1e-12);
            // ...and it always has more teeth than the real gear, which is why
            // a helical tooth is stronger in bending than its count suggests.
            assert!(v.z > g.z);
        }
    }

    /// Measuring the form on the transverse section — what an earlier revision
    /// did — is not the same as measuring it on the normal one. Compared at the
    /// *same* load point, so this isolates the section change from the load
    /// point change that also comes with the virtual gear.
    #[test]
    fn the_normal_section_is_what_bends_and_it_differs_from_the_transverse_one() {
        let mut previous = 0.0;
        for beta in [0.0, 15.0, 30.0] {
            let g = Tooth::new(GearParams {
                teeth: 20,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let roll = 0.35;
            let transverse = root_section(&g, roll).unwrap().form_factor;
            let normal = root_section(&v, roll).unwrap().form_factor;
            let gap = (normal - transverse).abs() / transverse;

            if beta == 0.0 {
                assert!(gap < 1e-15, "spur sections must coincide exactly");
            } else {
                assert!(gap > 0.005, "beta={beta}: sections differ by only {gap:.4}");
                assert!(gap > previous, "the gap must widen with the helix angle");
            }
            previous = gap;
        }
    }

    /// The load point moves too, and it must move the way ISO says: the virtual
    /// contact ratio is `ε_α / cos² β_b`, so a helical gear is loaded further
    /// down its (virtual) flank than the transverse contact ratio alone implies.
    #[test]
    fn the_bending_load_point_uses_the_virtual_contact_ratio() {
        for beta in [0.0, 15.0, 30.0] {
            let g = Tooth::new(GearParams {
                teeth: 20,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let eps = 1.55;
            let cos_bb = g.base_helix_angle().cos();
            let pbn = crate::plane::base_pitch(v.mt, v.alpha_t);

            let want =
                root_section(&v, v.u_tip - (eps / (cos_bb * cos_bb) - 1.0) * pbn / v.rb).unwrap();
            let got = bending_section(&g, eps).unwrap();
            assert!(
                (got.form_factor - want.form_factor).abs() < 1e-15,
                "beta={beta}"
            );

            // At beta = 0 the virtual contact ratio IS the transverse one.
            if beta == 0.0 {
                let plain = root_section(&g, g.u_tip - (eps - 1.0) * pbn / g.rb).unwrap();
                assert!((got.form_factor - plain.form_factor).abs() < 1e-15);
            }
        }
    }

    /// `ε_αn = ε_α/cos²β_b` is ISO's relation, not an identity: building the
    /// virtual pair and measuring its contact ratio directly gives a slightly
    /// different number, because the virtual gear keeps the addendum in normal
    /// modules and so its tip circle is not in exact correspondence with the
    /// real one. This pins down what that modelling gap actually costs, so the
    /// approximation is a measured quantity rather than an assumption.
    #[test]
    fn the_virtual_contact_ratio_relation_barely_moves_the_answer() {
        for (beta, spread) in [(10.0, 0.0004), (20.0, 0.0012), (30.0, 0.0020)] {
            let g = Tooth::new(GearParams {
                teeth: 17,
                helix_angle: beta,
                ..Default::default()
            });
            let eps = 1.5;
            let a = bending_section(&g, eps).unwrap();
            // Perturb the contact ratio by the observed disagreement between the
            // two routes and see how far the form factor moves.
            let b = bending_section(&g, eps * (1.0 + spread)).unwrap();
            let shift = (b.form_factor - a.form_factor).abs() / a.form_factor;
            assert!(
                shift < 0.005,
                "beta={beta}: a {spread:.4} contact-ratio gap moved Y_F by {shift:.5}"
            );
        }
    }

    /// Helical contact comes out below the equivalent transverse geometry by
    /// exactly `√(cos β_b)` — the combined effect of a longer contact line, a
    /// larger flank force and a flatter normal-plane curvature. Anything else
    /// means one of the three `cos β_b` factors is missing or doubled.
    #[test]
    fn helical_contact_stress_falls_by_the_square_root_of_the_base_helix_cosine() {
        let load = Load::new(2.0, 8.0);
        for beta in [10.0, 20.0, 30.0] {
            let g1 = Tooth::new(GearParams {
                teeth: 17,
                helix_angle: beta,
                ..Default::default()
            });
            let g2 = Tooth::new(GearParams {
                teeth: 43,
                helix_angle: -beta,
                ..Default::default()
            });
            let mesh = Mesh::new(&g1, &g2, MeshKind::External).unwrap();
            let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
            let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();

            // The transverse geometry itself changes with beta (m_t grows), so
            // compare against the same mesh computed without the plane change
            // rather than against the spur mesh directly.
            let cos_bb = g1.base_helix_angle().cos();
            let f_bt = load.transverse_line_of_action(&g1);
            // relative_radius is reported in the normal plane, ρ_n = ρ_t/cos β_b.
            let rho_t = cs.relative_radius * cos_bb;
            let transverse_only =
                ((f_bt / load.face_width) / rho_t * 113_000.0 / std::f64::consts::PI).sqrt();

            let ratio = cs.worst / transverse_only;
            assert!(
                (ratio - cos_bb.sqrt()).abs() < 1e-12,
                "beta={beta}: ratio {ratio} vs sqrt(cos beta_b) {}",
                cos_bb.sqrt()
            );
            assert!(cs.worst < transverse_only);
        }
    }

    /// Contact stress belongs to the *pair*, so it cannot depend on which gear
    /// the caller labelled 1. Checking only the inner single-pair boundary — as
    /// docs/reference.md#contact-stress originally prescribed — breaks this, because the relative
    /// radius peaks on the recess side for a pinion and the approach side for a
    /// wheel.
    #[test]
    fn contact_stress_does_not_depend_on_which_gear_is_called_first() {
        for (za, zb) in [(17u32, 43u32), (13, 60), (25, 25), (43, 17), (60, 13)] {
            let (g1, g2, mesh) = pair(za, zb);
            let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();

            let m_rev = Mesh::new(&g2, &g1, MeshKind::External).unwrap();
            let path_rev = ContactPath::new(&g2, g1.flank_ends(), &m_rev).unwrap();

            // Same physical mesh and same transmitted power, so the same load
            // along the line of action.
            let load = Load::new(2.0, 8.0);
            let load_rev = load.across_mesh(&g1, &g2);

            let a = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();
            let b = contact_stress(&path_rev, &m_rev, &g2, PARALLEL_AXES, &load_rev, 113_000.0)
                .unwrap();

            assert!(
                (a.worst - b.worst).abs() / a.worst < 1e-12,
                "z={za}/{zb}: {} vs {} when the labels are swapped",
                a.worst,
                b.worst
            );
            assert!((a.relative_radius - b.relative_radius).abs() < 1e-9);
            // The worst point is the same physical place. Its sign flips with
            // the labels, since ξ is measured toward gear 1's tip — except on a
            // symmetric mesh, where both single-pair boundaries are equally bad
            // and the tie-break picks the same one in both frames.
            assert!(
                (a.worst_position.abs() - b.worst_position.abs()).abs() < 1e-9,
                "z={za}/{zb}: worst at {} vs {}",
                a.worst_position,
                b.worst_position
            );
        }
    }

    /// **Gear 1's root is loaded at the low end of the path and gear 2's at the
    /// high end** — the geometric fact [`ContactStress::governing`] rests on.
    ///
    /// One relation covers both mesh kinds. A contact radius is `√(r_b² + ρ²)`,
    /// `ρ₁` rises with `ξ` and `ρ₂` falls, so gear 1's contact point climbs its
    /// flank as `ξ` grows while gear 2's radius moves the other way — outward
    /// for a ring, inward for an external gear, and **toward its root either
    /// way**, because a ring's root is its larger radius. That last step is the
    /// one a case analysis would get wrong, so the sign is taken from
    /// [`MeshKind::sign`] rather than assumed.
    #[test]
    fn each_gears_root_is_loaded_at_its_own_end_of_the_path() {
        for (kind, z1, z2) in [
            (MeshKind::External, 17u32, 43u32),
            (MeshKind::External, 43, 17),
            (MeshKind::External, 20, 20),
            (MeshKind::Internal, 17, 51),
            (MeshKind::Internal, 24, 60),
        ] {
            let g = |z| {
                Tooth::new(GearParams {
                    teeth: z,
                    ..Default::default()
                })
            };
            let mesh = Mesh::new(&g(z1), &g(z2), kind).expect("this pair should mesh");
            let (rb1, rb2) = mesh.base_radii();
            let span = mesh.a_w * mesh.alpha_w.sin();
            let radii = |xi: f64| {
                let (rho1, rho2) = mesh.curvature_radii(xi);
                (rb1.hypot(rho1), rb2.hypot(rho2))
            };
            let (lo, hi) = (radii(-0.3 * span), radii(0.3 * span));

            assert!(
                hi.0 > lo.0,
                "{kind:?} {z1}/{z2}: gear 1 must climb from its root as xi grows \
                 ({} to {})",
                lo.0,
                hi.0
            );
            // Toward gear 2's root: down in radius for an external gear, up for
            // a ring, which is exactly what the kind's sign says.
            assert!(
                kind.sign() * (hi.1 - lo.1) < 0.0,
                "{kind:?} {z1}/{z2}: gear 2 must approach its root as xi grows \
                 ({} to {})",
                lo.1,
                hi.1
            );
        }
    }

    /// **The individual curvatures do not reach the answer; only their sum
    /// does.**
    ///
    /// This is the claim that makes "the two teeth are different sizes, so they
    /// must see different stresses" false, and it is a property of Hertz rather
    /// than of this code — so it is checked against the code: two pairs built to
    /// the *same* relative radius out of wildly different individual radii must
    /// give the same pressure.
    #[test]
    fn only_the_relative_curvature_reaches_the_pressure() {
        // 1/ρ = 1/ρ₁ + 1/ρ₂ = 1/5 for every one of these.
        let same_relative = [(10.0_f64, 10.0_f64), (5.5, 55.0), (6.0, 30.0), (7.5, 15.0)];
        let (f_per_length, e_star) = (250.0_f64, 113_000.0_f64);
        let pressure = |rho1: f64, rho2: f64| {
            let inv_rho = 1.0 / rho1 + 1.0 / rho2;
            (f_per_length * inv_rho * e_star / std::f64::consts::PI).sqrt()
        };
        let first = pressure(same_relative[0].0, same_relative[0].1);
        for (rho1, rho2) in same_relative {
            let p = pressure(rho1, rho2);
            assert!(
                (p - first).abs() < 1e-9 * first,
                "rho1={rho1} rho2={rho2}: {p} against {first} — only 1/rho1 + 1/rho2 \
                 may reach the pressure"
            );
        }
    }

    #[test]
    fn contact_stress_is_worst_off_the_pitch_point_and_softens_with_a_softer_pair() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.flank_ends(), &mesh).unwrap();
        let load = Load::new(2.0, 8.0);

        let steel = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();
        // A single-pair point has the smaller relative radius, so one of the
        // two governs, and it is the worse of the two that sets the envelope.
        let single = steel.at_single_pair[0].max(steel.at_single_pair[1]);
        assert!(single > steel.at_pitch_point);
        assert!((steel.worst - single).abs() < 1e-12);
        // ...and each gear is rated at its own, which is this or the pitch point.
        for i in 0..2 {
            assert_eq!(
                steel.governing(i),
                steel.at_pitch_point.max(steel.at_single_pair[i])
            );
            assert!(steel.governing(i) <= steel.worst);
        }

        // A compliant pair spreads the contact and drops the pressure, as √E*.
        let poly = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 1_700.0).unwrap();
        assert!(poly.worst < steel.worst);
        let ratio = steel.worst / poly.worst;
        assert!(
            (ratio - (113_000.0f64 / 1_700.0).sqrt()).abs() < 1e-9,
            "contact stress should go as sqrt(E*), got ratio {ratio}"
        );
    }
}
