//! Involute gear geometry and geartrain mathematics.
//!
//! Pure computation: no I/O, no UI, no wasm. Every output is a function of its
//! inputs, so the whole crate is testable without a browser in the loop and the
//! same code serves the CLI, the test suite and the WebAssembly boundary.
//!
//! Design rationale, the mathematics behind each formula, and the verification
//! log live in `docs/reference.md`, `docs/rationale.md` and
//! `docs/corrections.md`.
//!
//! ```
//! use gear_core::{Gear, GearParams, Tooth};
//!
//! // A tooth is one tooth's form; a gear is the assembly that repeats it.
//! let t = Tooth::new(GearParams { teeth: 17, profile_shift: 0.2, ..Default::default() });
//! assert!(!t.undercut);
//! let outline = Gear::new(t.params).profile(400);   // closed cross-section, CCW
//! ```

pub mod auto;
pub mod contact;
pub mod elliptic;
pub mod gear;
pub mod hertz;
pub mod involute;
pub mod jgma;
pub mod material;
pub mod mesh;
pub mod metrology;
pub mod note;
pub mod outline;
pub mod params;
pub mod planetary;
pub mod ring;
pub mod screw;
pub mod shaper;
pub mod solve;
pub mod strength;
pub mod tooth;
pub mod train;
pub mod verify;

pub use gear::Gear;
pub use involute::{inv, inv_from_roll, inv_inverse};
pub use material::{Material, MaterialLibrary};
pub use mesh::{Mesh, MeshError, MeshKind};
pub use outline::Vertex;
pub use params::{Auto, Clamps, GearParams};
pub use tooth::{Section, Tooth};
