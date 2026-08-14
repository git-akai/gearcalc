//! File formats for the gear tool.
//!
//! Kept apart from `gear-core` so the mathematics has no notion of files, and
//! apart from the UI so a format can be tested without a browser.

pub mod dxf;

pub use dxf::{gear_to_dxf, DxfOptions};
