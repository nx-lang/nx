//! Public Rust API helpers intended to be shared across language bindings.
//!
//! This crate provides:
//! - [`eval_source`]: evaluate NX source text to a stable [`NxValue`](nx_value::NxValue)
//! - [`NxDiagnostic`]: a stable, serde-friendly diagnostic model for tooling and FFI
//! - [`to_nx_value`]: convert interpreter [`Value`](nx_interpreter::Value) to [`NxValue`](nx_value::NxValue)

mod diagnostics;
mod eval;
mod value;

pub use diagnostics::{NxDiagnostic, NxDiagnosticLabel, NxSeverity, NxTextSpan};
pub use eval::{eval_source, EvalResult};
pub use value::to_nx_value;

