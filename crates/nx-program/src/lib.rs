//! Shared NX program construction and resolved-program model.

mod resolved_program;

pub use resolved_program::{
    ModuleQualifiedExprRef, ModuleQualifiedItemRef, ResolvedItemKind, ResolvedModule,
    ResolvedModuleSource, ResolvedProgram, RuntimeModuleId,
};

mod artifacts;
mod diagnostics;
mod program_visualizers;
mod schema;
mod source_graph;
mod workspace;

#[cfg(test)]
mod library_source_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod standard_library_tests;
#[cfg(test)]
mod visualizer_declarations_tests;

pub use artifacts::{
    analyze_workspace_modules, build_library_artifact_from_directory,
    build_program_artifact_from_source, build_workspace_program_artifact, prelude_library,
    standard_libraries, standard_library, standard_library_entry, standard_library_for_module,
    unknown_standard_library_message, validate_workspace, ConstructedDeclaration, LibraryArtifact,
    LibraryExport, LibraryRegistry, NxLibraryModule, NxLibrarySource, ProgramArtifact,
    ProgramBuildContext, ProgramSourceEntry, StandardLibrary, StandardLibraryModule,
    StandardLibraryStability,
};
pub use diagnostics::{
    diagnostics_to_api_with_source_entries, NxDiagnostic, NxDiagnosticLabel, NxSeverity, NxTextSpan,
};
pub use program_visualizers::{
    duplicate_program_visualizers, duplicate_visualizer_diagnostics, program_modules,
    program_visualizers, read_visualizer_value, ProgramModule, ProgramVisualizer,
    VisualizerAttribute, VisualizerChild, VisualizerChoice, VisualizerValue,
};
pub use schema::{
    program_artifact_boundary_subtype_modules, program_artifact_function_schema,
    program_artifact_function_schema_json, program_artifact_type_schema,
    program_artifact_type_schema_json, DeclarationName, DeclarationRef, FunctionSchema,
    FunctionSchemaOptions, ParameterSchema, SchemaDirection, SchemaQueryError, SchemaRequest,
    SchemaValue, TypeSchema, TypeSchemaOptions, JSON_SCHEMA_DIALECT,
    SCHEMA_AMBIGUOUS_DISCRIMINATOR, SCHEMA_INEXPRESSIBLE_TYPE, SCHEMA_UNKNOWN_DECLARATION,
};
pub use workspace::{
    NxWorkspace, NxWorkspaceDirectoryError, NxWorkspaceInputError, NxWorkspaceModule,
};

pub use diagnostics::diagnostics_to_api;

pub use diagnostics::LineIndex;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
