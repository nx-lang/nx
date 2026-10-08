//! Executable TypeScript and JavaScript generation for resolved NX programs.
//!
//! This crate consumes [`nx_api::ProgramArtifact`] values. It does not parse source text or
//! rediscover imports during emission; all executable generation starts from the resolved,
//! type-checked program model preserved by `nx-api`.

// The builders and emitters thread a wide, consistent context — the program, the module, the
// declaration, the emit options, the export policy and the output buffer — through one function
// per construct rather than one method on a shared struct. Splitting a signature here would move
// the arguments into a struct built at every call site and nothing else, so the lint is allowed
// for the crate.
#![allow(clippy::too_many_arguments)]

mod builder;
mod emit;
mod ir;
mod ir_bundle;
mod model;
mod options;
mod runtime;

pub use builder::build_codegen_program;
pub use emit::{
    emit_codegen_js_program_module, emit_codegen_program, emit_js_program_module, emit_program,
};
pub use ir::{
    build_nx_ir_artifacts, emit_codegen_nx_ir, emit_nx_ir, nx_ir_metadata, GeneratedNxIr,
    NxIrEmitOptions, NxIrMetadata,
};
pub use ir_bundle::{read_nx_ir_bundle, write_nx_ir_bundle, NxIrBundleEntry};
pub use model::{
    CodegenActionHandler, CodegenComponent, CodegenComponentDescriptor, CodegenComponentEmit,
    CodegenComponentField, CodegenDeclaration, CodegenDeclarationKind, CodegenElement,
    CodegenEntrypoint, CodegenExpression, CodegenExpressionKind, CodegenMatchArm, CodegenModule,
    CodegenModuleProvenance, CodegenProgram, CodegenReference, CodegenSourceEntry, CodegenTypeRef,
    CodegenUnsupportedConstruct,
};
pub use options::{
    CodegenError, CodegenOptions, CodegenOutput, CodegenOutputFormat, CodegenTarget,
    CodegenWarning, GeneratedFile, GeneratedJsProgramModule,
    GeneratedJsProgramModuleComponentExport, GeneratedJsProgramModuleFunctionExport,
    JsProgramModuleOptions, DEFAULT_JS_PROGRAM_MODULE_NAME,
    DEFAULT_JS_PROGRAM_MODULE_RUNTIME_IMPORT_SPECIFIER, NX_JS_RUNTIME_ABI,
};
pub use runtime::{javascript_runtime_abi, javascript_runtime_helper_source};

#[cfg(test)]
mod ir_corpus_tests;
#[cfg(test)]
mod ir_image_tests;
#[cfg(test)]
mod ir_instance_tree_tests;
#[cfg(test)]
mod ir_runtime_sources;
#[cfg(test)]
mod ir_runtime_tests;
#[cfg(test)]
mod ir_tests;
#[cfg(test)]
mod prelude_image_tests;
#[cfg(test)]
mod tests;
