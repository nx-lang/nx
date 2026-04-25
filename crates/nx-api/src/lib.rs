//! Public Rust API helpers intended to be shared across language bindings.
//!
//! This crate provides:
//! - [`eval_source`]: evaluate NX source text to a stable [`NxValue`](nx_value::NxValue)
//! - [`eval_program_artifact`]: evaluate the `root()` entrypoint of a previously built
//!   [`ProgramArtifact`]
//! - [`NxWorkspace`] / [`NxWorkspaceModule`]: validate and build programs from logical in-memory
//!   source modules without temporary files
//! - [`initialize_component_source`] / [`dispatch_component_actions_source`]: component lifecycle
//!   entry points that analyze source against a caller-supplied [`ProgramBuildContext`],
//!   initialize a named component, and dispatch action batches
//! - [`initialize_component_program_artifact`] / [`dispatch_component_actions_program_artifact`]:
//!   component lifecycle entry points that execute a resolved [`ProgramArtifact`]
//! - [`NxDiagnostic`]: a stable, serde-friendly diagnostic model for tooling and FFI
//! - [`to_nx_value`] / [`from_nx_value`]: convert between interpreter
//!   [`Value`](nx_interpreter::Value) and [`NxValue`](nx_value::NxValue), rejecting runtime-only
//!   callback values on the reverse path
//!
//! Workspace module identities use forward-slash logical path semantics. NX normalizes `.` and
//! `..`, rejects absolute or root-escaping identities, resolves workspace imports by exact
//! normalized identity before consulting [`ProgramBuildContext`], and maps diagnostics from the
//! submitted source text before any file-backed fallback. Workspace modules validate UTF-8 at
//! construction and share decoded source text internally.

mod artifacts;
mod component;
mod diagnostics;
mod eval;
mod source_graph;
mod value;
mod workspace;

pub use artifacts::{
    build_library_artifact_from_directory, build_program_artifact_from_source,
    build_workspace_program_artifact, validate_workspace, LibraryArtifact, LibraryExport,
    LibraryRegistry, ProgramArtifact, ProgramBuildContext,
};
pub use component::{
    dispatch_component_actions_program_artifact, dispatch_component_actions_source,
    initialize_component_program_artifact, initialize_component_source,
    ComponentDispatchEvalResult, ComponentDispatchResult, ComponentInitEvalResult,
    ComponentInitResult,
};
pub use diagnostics::{NxDiagnostic, NxDiagnosticLabel, NxSeverity, NxTextSpan};
pub use eval::{
    eval_program_artifact, eval_source, load_library_artifact_from_directory,
    load_program_artifact_from_source, EvalResult,
};
pub use value::{from_nx_value, to_nx_value, FromNxValueError};
pub use workspace::{NxWorkspace, NxWorkspaceInputError, NxWorkspaceModule};
