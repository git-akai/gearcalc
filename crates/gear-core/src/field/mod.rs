//! The contact field: the expensive mode's contact model. Each tooth pair's line of contact is
//! traced through the exact gap of two generated flanks and solved with the teeth's compliance.
//!
//! Additive: the fast mode reads nothing here. It is ported from the prototype step by step, and
//! each step reproduces its records in `tests/data/field_oracle/` (the porting plan is
//! `work/contact-model.md`, "Stage 3: the Rust port").
//!
//! - [`form`]: a gear's tip form, the relief and the edge round on the flank ([`form::ToothForm`],
//!   laid on its corner as a [`form::TipForm`]).
//! - [`gap`]: a gear as the field reads it ([`gap::FieldGear`]).

pub mod form;
pub mod gap;

#[cfg(test)]
pub(crate) mod oracle;
