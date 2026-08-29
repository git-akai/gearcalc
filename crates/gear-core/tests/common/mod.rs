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
