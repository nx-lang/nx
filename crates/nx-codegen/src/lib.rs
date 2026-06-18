//! Executable TypeScript and JavaScript generation for resolved NX programs.
//!
//! This crate consumes [`nx_api::ProgramArtifact`] values. It does not parse source text or
//! rediscover imports during emission; all executable generation starts from the resolved,
//! type-checked program model preserved by `nx-api`.

mod builder;
mod emit;
mod model;
mod options;
mod runtime;

pub use builder::build_codegen_program;
pub use emit::{
    emit_codegen_js_program_module, emit_codegen_program, emit_js_program_module, emit_program,
};
pub use model::{
    CodegenComponent, CodegenComponentDescriptor, CodegenComponentField, CodegenDeclaration,
    CodegenDeclarationKind, CodegenElement, CodegenEntrypoint, CodegenExpression,
    CodegenExpressionKind, CodegenModule, CodegenModuleProvenance, CodegenProgram,
    CodegenReference, CodegenSourceEntry, CodegenUnsupportedConstruct,
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
mod tests;
