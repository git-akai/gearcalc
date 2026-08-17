//! Involute gear geometry and geartrain mathematics.
//!
//! Pure computation: no I/O, no UI, no wasm. Every output is a function of its
//! inputs, so the whole crate is testable without a browser in the loop and the
//! same code serves the CLI, the test suite and the WebAssembly boundary.
//!
//! Design rationale, the mathematics behind each formula, and the verification
//! log live in `docs/DESIGN.md`.
//!
//! ```
//! use gear_core::{Gear, GearParams};
//!
//! let g = Gear::new(GearParams { teeth: 17, profile_shift: 0.2, ..Default::default() });
//! assert!(!g.undercut);
//! let outline = g.profile(400);        // closed cross-section, CCW
//! ```

pub mod auto;
pub mod contact;
pub mod involute;
pub mod jgma;
pub mod material;
pub mod mesh;
pub mod metrology;
pub mod outline;
pub mod params;
pub mod profile;
pub mod solve;
pub mod strength;
pub mod train;
pub mod verify;

pub use involute::{inv, inv_from_roll, inv_inverse};
pub use material::{Material, MaterialLibrary};
pub use mesh::{Mesh, MeshError, MeshKind};
pub use outline::Vertex;
pub use params::{Auto, Clamps, GearParams};
pub use profile::{Gear, Section};
