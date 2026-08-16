//! File formats for the gear tool.
//!
//! Kept apart from `gear-core` so the mathematics has no notion of files, and
//! apart from the UI so a format can be tested without a browser.

pub mod dxf;
pub mod materials;

pub use dxf::{gear_to_dxf, DxfOptions};
pub use materials::{default_library, from_toml, to_toml, MaterialError};
