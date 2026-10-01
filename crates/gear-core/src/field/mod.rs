//! The contact field: the expensive mode's contact model. Each tooth pair's line of contact is
//! traced through the exact gap of two generated flanks and solved with the teeth's compliance.
//!
//! Additive: the fast mode reads nothing here. It is ported from the prototype step by step, and
//! each step reproduces its records in `tests/data/field_oracle/` (the porting plan is
//! `work/contact-model.md`, "Stage 3: the Rust port").
//!
//! - [`across`]: the frictionless contact of one strip across the line, read by regions, its
//!   exact creep curve, and a panel's friction ([`across::Strip`], [`across::StripReport`]).
//! - [`form`]: a gear's tip form, the relief and the edge round on the flank ([`form::ToothForm`],
//!   laid on its corner as a [`form::TipForm`]).
//! - [`gap`]: a gear as the field reads it ([`gap::FieldGear`]).
//! - [`kernel`]: how a line load on one panel of a contact line moves a body's surface at a point
//!   of it ([`kernel::Kernel`], a [`kernel::Panel`] on a [`kernel::HalfSpace`]).

pub mod across;
pub mod form;
pub mod gap;
pub mod kernel;

#[cfg(test)]
pub(crate) mod oracle;
#[cfg(test)]
pub(crate) mod wide;
