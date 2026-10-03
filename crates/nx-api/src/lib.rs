//! Public Rust API helpers intended to be shared across language bindings.
//!
//! This crate provides:
//! - [`eval_source`]: evaluate NX source text to a stable [`NxValue`](nx_value::NxValue)
//! - [`eval_program_artifact`]: evaluate the `root()` entrypoint of a previously built
//!   [`ProgramArtifact`]
//! - [`eval_program_artifact_nx_text`] / [`format_nx_text`]: spell a value as NX text, the form
//!   `nxlang run` prints, optionally annotated with what each part of the text is
//! - [`evaluate_component_source`] / [`evaluate_component_program_artifact`]: pure component
//!   rendering from explicit props and host-owned current state, returning the rendered value
//!   directly without lifecycle wrapper fields
//! - [`NxWorkspace`] / [`NxWorkspaceModule`]: validate and build programs from logical in-memory
//!   source modules or load a logical workspace from a directory
//! - [`initialize_component_source`] / [`dispatch_component_actions_source`]: component lifecycle
//!   entry points that analyze source against a caller-supplied [`ProgramBuildContext`],
//!   initialize a named component, and dispatch action batches
//! - [`initialize_component_program_artifact`] / [`dispatch_component_actions_program_artifact`]:
//!   component lifecycle entry points that execute a resolved [`ProgramArtifact`]
//! - [`program_artifact_function_schema`] / [`program_artifact_type_schema`]: JSON Schema for a
//!   function's arguments and result, or for a declared type, with `///` documentation as
//!   descriptions
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
mod nx_text;
mod schema;
mod source_graph;
mod value;
mod workspace;

#[cfg(test)]
mod library_source_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod standard_library_tests;

pub use artifacts::{
    analyze_workspace_modules, build_library_artifact_from_directory,
    build_program_artifact_from_source, build_workspace_program_artifact, prelude_library,
    standard_libraries, standard_library, standard_library_entry, standard_library_for_module,
    unknown_standard_library_message, validate_workspace, LibraryArtifact, LibraryExport,
    LibraryRegistry, NxLibraryModule, NxLibrarySource, ProgramArtifact, ProgramBuildContext,
    ProgramSourceEntry, StandardLibrary, StandardLibraryModule, StandardLibraryStability,
};
pub use component::{
    dispatch_component_actions_program_artifact, dispatch_component_actions_source,
    evaluate_component_program_artifact, evaluate_component_source,
    initialize_component_program_artifact, initialize_component_source,
    ComponentDispatchEvalResult, ComponentDispatchResult, ComponentEvaluateEvalResult,
    ComponentEvaluateResult, ComponentInitEvalResult, ComponentInitResult,
};
pub use diagnostics::{
    diagnostics_to_api_with_source_entries, NxDiagnostic, NxDiagnosticLabel, NxSeverity, NxTextSpan,
};
pub use eval::{
    eval_program_artifact, eval_program_artifact_function, eval_program_artifact_nx_text,
    eval_program_artifact_nx_text_with_limits, eval_source, load_library_artifact_from_directory,
    load_program_artifact_from_source, EvalResult,
};
pub use nx_interpreter::ResourceLimits;
pub use nx_text::{format_nx_text, NxValueNode, NxValueRole, NxValueText};
pub use schema::{
    program_artifact_boundary_subtype_modules, program_artifact_function_schema,
    program_artifact_function_schema_json, program_artifact_type_schema,
    program_artifact_type_schema_json, DeclarationName, DeclarationRef, FunctionSchema,
    FunctionSchemaOptions, ParameterSchema, SchemaDirection, SchemaQueryError, SchemaRequest,
    SchemaValue, TypeSchema, TypeSchemaOptions, JSON_SCHEMA_DIALECT,
    SCHEMA_AMBIGUOUS_DISCRIMINATOR, SCHEMA_INEXPRESSIBLE_TYPE, SCHEMA_UNKNOWN_DECLARATION,
};
pub use value::{entry_result_to_nx_value, from_nx_value, to_nx_value, FromNxValueError};
pub use workspace::{
    NxWorkspace, NxWorkspaceDirectoryError, NxWorkspaceInputError, NxWorkspaceModule,
};
