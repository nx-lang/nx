//! External Rust entry point for NX tooling and language bindings.
//!
//! Program construction and inspection come from `nx-program`. Code generation, editor queries
//! and IR execution are available through [`codegen`], [`language_service`] and [`ir_runtime`].
//! Workspace module identities use forward-slash logical path semantics. NX normalizes `.` and
//! `..`, rejects absolute or root-escaping identities, resolves workspace imports by exact
//! normalized identity before consulting [`ProgramBuildContext`], and maps diagnostics from the
//! submitted source text before any file-backed fallback. Workspace modules validate UTF-8 at
//! construction and share decoded source text internally.

mod eval;
mod format_value;
mod nx_text;

pub use eval::{
    format_program_artifact_function_value, format_program_artifact_value,
    load_library_artifact_from_directory, load_program_artifact_from_source, FormatValueError,
};
pub use nx_text::{NxValueNode, NxValueRole, NxValueText};

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

#[cfg(test)]
extern crate self as nx_api;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
