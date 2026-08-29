//! Gear input parameters, and the record of any guard that altered them.

/// Input parameters for a single gear.
///
/// Angles are **degrees** here, because this is the boundary the UI writes to.
/// Everything downstream works in radians.
///
/// `addendum`, `dedendum` and `root_radius` are multiples of the **normal**
/// module. `profile_shift` likewise.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct GearParams {
    /// Normal module, mm.
    pub module: f64,
    /// Normal pressure angle, degrees.
    pub pressure_angle: f64,
    pub teeth: u32,
    /// Profile shift `x`, in modules.
    pub profile_shift: f64,
    /// Helix angle, degrees. Sign selects hand.
    pub helix_angle: f64,
    /// Addendum, in modules.
    pub addendum: f64,
    /// Dedendum, in modules.
    pub dedendum: f64,
    /// Cutter tip radius, in modules. 0.38 is the ISO 53 basic rack.
    pub root_radius: f64,
    /// Tooth thickness modification `k`, dimensionless, nominally 1.
    ///
    /// Defined on the rack: tooth width `(π m/2)·k`, space width
    /// `(π m/2)·(2−k)`, so the pitch is preserved. Two gears in mesh must sum to
    /// 2 for zero backlash.
    ///
    /// This is *not* separate geometry — it is exactly an extra thickness-only
    /// profile shift, `x_s = π(k−1)/(4 tan αₙ)`. See [`GearParams::thickness_shift`].
    pub thickness_mod: f64,
    /// Amplitude of an **angularly varying** profile shift, in modules.
    ///
    /// `x(θ) = profile_shift + angular_shift · cos θ`, maximum at 0° and minimum
    /// at 180° — what a hob moving radially in and out once per revolution
    /// produces. It makes the tip and root envelopes eccentric by
    /// `e = module · angular_shift` while the pitch and base circles stay on the
    /// axis, so the body moves eccentrically at a constant ratio (docs/reference.md#angularly-varying-profile-shift).
    ///
    /// **Zero is an ordinary gear**, and not by a branch: every tooth then takes
    /// the same shift and the whole construction collapses onto the single-`x`
    /// one it extends, bit for bit.
    #[cfg_attr(feature = "serde", serde(default))]
    pub angular_shift: f64,
    /// `λ`, how far the indexing compensates for the varying tooth thickness.
    ///
    /// A gear with varying tooth thickness cannot be exactly conjugate in both
    /// directions — uniform spacing on both flanks forces uniform thickness, in
    /// two lines of algebra (docs/reference.md#angularly-varying-profile-shift) — so the unavoidable error can only be
    /// distributed. Tooth `k` is seated at `2πk/z + λ(ψ_b,ref − ψ_b,k)`, which
    /// scales the drive-flank error by `|1 − λ|` and the coast-flank error by
    /// `|1 + λ|`.
    ///
    /// `0` is the minimax optimum and what a plain radial hob oscillation gives;
    /// `1` is exactly conjugate forward and twice the error in reverse, and needs
    /// the radial motion synchronised with a differential rotation of the work.
    /// It has no effect at all when [`Self::angular_shift`] is zero, since every
    /// tooth then has the same seat to correct towards.
    #[cfg_attr(feature = "serde", serde(default))]
    pub index_offset: f64,
}

impl Default for GearParams {
    fn default() -> Self {
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            teeth: 17,
            profile_shift: 0.0,
            helix_angle: 0.0,
            addendum: 1.0,
            dedendum: 1.25,
            root_radius: 0.38,
            thickness_mod: 1.0,
            angular_shift: 0.0,
            index_offset: 0.0,
        }
    }
}

impl GearParams {
    /// The equivalent thickness-only profile shift of `thickness_mod`.
    ///
    /// `x_s = π (k − 1) / (4 tan αₙ)`, so that
    /// `s_n = m (π/2 + 2(x + x_s) tan αₙ)` reproduces
    /// `s_n = m ((π/2)k + 2x tan αₙ)` identically.
    ///
    /// The consequence used throughout: **radial** quantities (root radius, tip
    /// radius, cutter depth) take `x`, while **thickness** quantities take
    /// `x + x_s`. No other change is needed anywhere to support thickness
    /// modification.
    #[must_use]
    pub fn thickness_shift(&self) -> f64 {
        let an = self.pressure_angle.to_radians();
        std::f64::consts::PI * (self.thickness_mod - 1.0) / (4.0 * an.tan())
    }

    /// Whether these two gears were cut by the same rack, so that they *could*
    /// mesh.
    ///
    /// The helix is deliberately **not** here. Whether two hands mesh depends on
    /// the kind of pair — external gears take opposite hands and an internal
    /// pair the same — so it is a question about the mesh rather than about
    /// either gear, and it belongs where the kind is known
    /// ([`crate::mesh::Mesh::new`]). What every mesh needs alike is the rack:
    /// the same module and the same pressure angle, or the teeth are simply not
    /// the same size.
    ///
    /// One function because there were three, each with its own tolerance and
    /// none with a stated reason for differing (`docs/corrections.md`). A
    /// duplicated condition is a place two answers can differ, and here two of
    /// them already did.
    #[must_use]
    pub fn same_rack_as(&self, other: &Self) -> bool {
        (self.module - other.module).abs() < compat::SAME_RACK
            && (self.pressure_angle - other.pressure_angle).abs() < compat::SAME_RACK
    }
}

/// A value the solver can work out for you, or that you can set yourself.
///
/// The specification has about a dozen of these — profile shift, altered
/// addendum, centre distance, face width — all with the same shape: a toggle,
/// and a field that is locked while the toggle is on. One generic covers them
/// all (`docs/rationale.md#inputs-are-the-only-state`).
///
/// `manual` is kept even while `auto` is set, so turning automatic *off* leaves
/// the field showing the last value rather than jumping to a stale one. It is
/// the UI's job to seed `manual` from the solved value when the toggle flips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Auto<T> {
    pub auto: bool,
    pub manual: T,
}

impl<T: Copy> Auto<T> {
    /// Automatic, with `manual` seeded to a sensible starting value.
    pub const fn automatic(seed: T) -> Self {
        Self {
            auto: true,
            manual: seed,
        }
    }

    /// Manual, at this value.
    pub const fn fixed(v: T) -> Self {
        Self {
            auto: false,
            manual: v,
        }
    }

    /// The value in force: `computed` when automatic, otherwise `manual`.
    ///
    /// Takes the computed value rather than a closure so the caller decides
    /// whether computing it is worth the work — several of these involve a
    /// solve.
    pub fn resolve(&self, computed: T) -> T {
        if self.auto {
            computed
        } else {
            self.manual
        }
    }
}

/// Guard rails applied to degenerate input.
///
/// These encode no physics. They stop input that cannot describe a real gear
/// from producing a NaN or a self-intersecting outline. Every guard that fires
/// appends a human-readable note, so a clamped result is never returned
/// silently — check [`Clamps::any`] when a number looks wrong, because the
/// geometry may not be the geometry that was asked for.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Clamps {
    pub notes: Vec<crate::note::Note>,
}

impl Clamps {
    pub fn push(&mut self, note: crate::note::Note) {
        self.notes.push(note);
    }

    /// Whether a named guard fired.
    ///
    /// Consumers that need to *act* on a clamp ask this. The planetary solve
    /// used to search the text instead — a sentence doing a symbol's job, and
    /// one improved wording away from silently doing nothing.
    #[must_use]
    pub fn fired(&self, key: &str) -> bool {
        self.notes.iter().any(|n| n.is(key))
    }

    #[must_use]
    pub fn any(&self) -> bool {
        !self.notes.is_empty()
    }
}

/// Limits for the guard rails in [`crate::tooth`].
///
/// Named and gathered so they are auditable in one place rather than scattered
/// as bare literals through the geometry. They are tolerances on *degeneracy*,
/// chosen to be far outside any real design, not tuning parameters that change
/// a valid result.
pub(crate) mod guard {
    /// Smallest pressure angle that still generates a usable flank, degrees.
    /// Below this the base circle approaches the pitch circle and the involute
    /// degenerates.
    pub const MIN_PRESSURE_ANGLE_DEG: f64 = 0.5;

    /// Smallest cutter depth, in modules. Zero depth means no tooth at all.
    pub const MIN_CUTTER_DEPTH_MODULES: f64 = 0.05;

    /// Largest cutter depth, as a fraction of the pitch radius. Beyond this the
    /// root radius reaches the centre.
    pub const MAX_CUTTER_DEPTH_FRACTION_OF_R: f64 = 0.9;

    /// Smallest transverse tooth thickness at the pitch circle, in modules.
    pub const MIN_TOOTH_THICKNESS_MODULES: f64 = 0.02;

    /// Largest transverse tooth thickness, as a fraction of the circular pitch.
    /// At 1.0 the teeth touch and there is no space left to mesh into.
    pub const MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH: f64 = 0.95;

    /// Fraction of the geometric maximum the cutter tip radius is allowed to
    /// reach. Sitting exactly on the limit leaves a zero-width root arc, which
    /// is legal but numerically awkward.
    pub const FILLET_FRACTION_OF_MAX: f64 = 0.95;

    /// Floor on the cutter tip radius, in modules. A truly sharp corner is a
    /// removable singularity in the trochoid; this keeps it finite.
    pub const MIN_FILLET_MODULES: f64 = 1e-9;

    /// How far above the base circle the tip radius is forced to sit, as a
    /// fraction of the base radius. Below the base circle there is no involute.
    pub const TIP_ABOVE_BASE_FRACTION: f64 = 1e-9;
}

/// How nearly two gears must agree before they are taken to be the same rack.
///
/// # Why these are gathered, and why there is only one of each
///
/// Three places asked "can these two mesh?" and each answered with its own bare
/// literal: `1e-12` on module and pressure angle in [`crate::mesh::Mesh::new`],
/// `1e-9` on the transverse module and pressure angle in
/// [`crate::ring::mesh_with`], `1e-12` again on an eccentric gear's mate. They
/// disagreed with each other, and no reason was written down anywhere for the
/// disagreement — which is the same arrangement `guard` above exists to avoid.
///
/// The values are **degeneracy tolerances, not design ones**: two gears whose
/// modules differ by anything a designer could mean do not mesh at all, so the
/// only job here is to admit values that arrived by different arithmetic —
/// `m_n/cos β` computed twice, a pressure angle round-tripped through degrees.
/// A handful of ulps on a quantity of order one is what that costs, and the
/// tolerance is generous against it by orders of magnitude.
pub mod compat {
    /// A length or a dimensionless coefficient, absolute. Modules are of order
    /// 1 mm and pressure angles of order 20°, so an absolute tolerance is the
    /// same claim at both.
    pub const SAME_RACK: f64 = 1e-9;
}
