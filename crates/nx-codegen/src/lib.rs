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
pub use emit::{emit_codegen_program, emit_program};
pub use model::{
    CodegenComponent, CodegenComponentDescriptor, CodegenComponentField, CodegenDeclaration,
    CodegenDeclarationKind, CodegenElement, CodegenEntrypoint, CodegenExpression,
    CodegenExpressionKind, CodegenModule, CodegenModuleProvenance, CodegenProgram,
    CodegenReference, CodegenSourceEntry, CodegenUnsupportedConstruct,
};
pub use options::{
    CodegenError, CodegenOptions, CodegenOutput, CodegenOutputFormat, CodegenTarget,
    CodegenWarning, GeneratedFile,
};

#[cfg(test)]
mod tests;
