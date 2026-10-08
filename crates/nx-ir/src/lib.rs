//! The NX IR format: its constants and kind numbers, the artifact model, the image writer, the
//! validating in-place reader and the explainer.
//!
//! This crate depends on no other NX crate. The emitter in `nx-codegen` writes images through it
//! and the runtime in `nx-ir-runtime` reads them through it, so the two share one implementation of
//! the layout `docs/nx-ir-format.md` documents.

mod explain;
mod image;
mod model;

pub use explain::{explain_nx_ir, explain_nx_ir_image, ExplainError};
pub use image::{
    section, write_nx_ir_image, Cells, NxIrImage, NxIrImageBuf, NxIrImageError, NxIrModuleRef,
    Table, NONE, NX_IR_MAGIC,
};
pub use model::{
    kinds, IrItem, NxIrArtifact, NxIrDebug, NxIrDebugSpans, NxIrModuleEntry,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1, NX_IR_REQUIRED_FEATURE_FUNCTION_REFERENCE_TYPE_V1,
    NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1, NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1,
    NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1, NX_IR_REQUIRED_FEATURE_RANGES_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1, NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
    NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION,
};

/// The README's example, compiled as a doc test.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
