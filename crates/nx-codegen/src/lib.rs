//! Executable TypeScript and JavaScript generation for resolved NX programs.
//!
//! This crate consumes [`nx_api::ProgramArtifact`] values. It does not parse source text or
//! rediscover imports during emission; all executable generation starts from the resolved,
//! type-checked program model preserved by `nx-api`.

mod builder;
mod emit;
mod ir;
mod model;
mod options;
mod runtime;

pub use builder::build_codegen_program;
pub use emit::{
    emit_codegen_js_program_module, emit_codegen_program, emit_js_program_module, emit_program,
};
pub use ir::{
    emit_codegen_nx_ir, emit_nx_ir, GeneratedNxIr, NxIrComponent, NxIrComponentField,
    NxIrDeclaration, NxIrDeclarationKind, NxIrEntrypoint, NxIrEntrypointMetadata, NxIrExpression,
    NxIrExpressionOp, NxIrLiteral, NxIrMatchArm, NxIrMetadata, NxIrModule, NxIrModuleProvenance,
    NxIrParam, NxIrProgram, NxIrProperty, NxIrRecordField, NxIrReference, NxIrSemanticType,
    NxIrSemanticTypeShape, NxIrSourceEntry, NxIrSourceSpan, NxIrStatement, NxIrTypeRef,
    NxIrUnionCase, NX_IR_FORMAT_ID, NX_IR_REQUIRED_FEATURE_EAGER_V1, NX_IR_RUNTIME_ABI,
    NX_IR_SCHEMA_VERSION,
};
pub use model::{
    CodegenComponent, CodegenComponentDescriptor, CodegenComponentField, CodegenDeclaration,
    CodegenDeclarationKind, CodegenElement, CodegenEntrypoint, CodegenExpression,
    CodegenExpressionKind, CodegenMatchArm, CodegenModule, CodegenModuleProvenance, CodegenProgram,
    CodegenReference, CodegenSourceEntry, CodegenTypeRef, CodegenUnsupportedConstruct,
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
