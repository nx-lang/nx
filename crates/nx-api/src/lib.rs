//! External Rust entry point for NX tooling and language bindings.
//!
//! Program construction and inspection come from `nx-program`. Code generation, editor queries
//! and IR execution are available through [`codegen`], [`language_service`] and [`ir_runtime`].
//! Flat evaluation functions continue to use the HIR interpreter.
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

mod component;
mod eval;
mod nx_text;
mod value;

pub use component::{
    dispatch_component_actions_program_artifact, dispatch_component_actions_source,
    evaluate_component_program_artifact, evaluate_component_source,
    initialize_component_program_artifact, initialize_component_source,
    ComponentDispatchEvalResult, ComponentDispatchResult, ComponentEvaluateEvalResult,
    ComponentEvaluateResult, ComponentInitEvalResult, ComponentInitResult,
};
pub use eval::{
    eval_program_artifact, eval_program_artifact_function, eval_program_artifact_nx_text,
    eval_program_artifact_nx_text_with_limits, eval_source, format_program_artifact_value,
    load_library_artifact_from_directory, load_program_artifact_from_source, EvalResult,
    FormatValueError,
};
pub use nx_interpreter::ResourceLimits;
pub use nx_text::{format_nx_text, NxValueNode, NxValueRole, NxValueText};
pub use value::{entry_result_to_nx_value, from_nx_value, to_nx_value, FromNxValueError};

pub use nx_program::{
    analyze_workspace_modules, build_library_artifact_from_directory,
    build_program_artifact_from_source, build_workspace_program_artifact,
    diagnostics_to_api_with_source_entries, duplicate_program_visualizers,
    duplicate_visualizer_diagnostics, prelude_library, program_artifact_boundary_subtype_modules,
    program_artifact_function_schema, program_artifact_function_schema_json,
    program_artifact_type_schema, program_artifact_type_schema_json, program_diagnostics_to_api,
    program_modules, program_visualizers, read_visualizer_value, standard_libraries,
    standard_library, standard_library_entry, standard_library_for_module,
    unknown_standard_library_message, validate_workspace, ConstructedDeclaration, DeclarationName,
    DeclarationRef, FunctionSchema, FunctionSchemaOptions, LibraryArtifact, LibraryExport,
    LibraryRegistry, ModuleQualifiedExprRef, ModuleQualifiedItemRef, NxDiagnostic,
    NxDiagnosticLabel, NxLibraryModule, NxLibrarySource, NxSeverity, NxTextSpan, NxWorkspace,
    NxWorkspaceDirectoryError, NxWorkspaceInputError, NxWorkspaceModule, ParameterSchema,
    ProgramArtifact, ProgramBuildContext, ProgramModule, ProgramSourceEntry, ProgramVisualizer,
    ResolvedItemKind, ResolvedModule, ResolvedModuleSource, ResolvedProgram, RuntimeModuleId,
    SchemaDirection, SchemaQueryError, SchemaRequest, SchemaValue, StandardLibrary,
    StandardLibraryModule, StandardLibraryStability, TypeSchema, TypeSchemaOptions,
    VisualizerAttribute, VisualizerChild, VisualizerChoice, VisualizerValue, JSON_SCHEMA_DIALECT,
    SCHEMA_AMBIGUOUS_DISCRIMINATOR, SCHEMA_INEXPRESSIBLE_TYPE, SCHEMA_UNKNOWN_DECLARATION,
};

/// Code generation for resolved programs.
pub use nx_codegen as codegen;
/// Execution of compiled NX IR.
pub use nx_ir_runtime as ir_runtime;
/// Editor analysis and queries.
pub use nx_language_service as language_service;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
