//! Executable TypeScript and JavaScript generation for resolved NX programs.
//!
//! This crate consumes [`nx_api::ProgramArtifact`] values. It does not parse source text or
//! rediscover imports during emission; all executable generation starts from the resolved,
//! type-checked program model preserved by `nx-api`.

mod builder;
mod emit;
mod ir;
mod ir_bundle;
mod ir_explain;
mod ir_image;
mod model;
mod options;
mod runtime;

pub use builder::build_codegen_program;
pub use emit::{
    emit_codegen_js_program_module, emit_codegen_program, emit_js_program_module, emit_program,
};
pub use ir::{
    build_nx_ir_artifacts, emit_codegen_nx_ir, emit_nx_ir, kinds as nx_ir_kinds, GeneratedNxIr,
    IrItem, NxIrArtifact, NxIrDebug, NxIrDebugSpans, NxIrEmitOptions, NxIrMetadata,
    NxIrModuleEntry, NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1, NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
    NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION,
};
pub use ir_bundle::{read_nx_ir_bundle, write_nx_ir_bundle, NxIrBundleEntry};
pub use ir_explain::{explain_nx_ir, explain_nx_ir_image, ExplainError};
pub use ir_image::{
    section as nx_ir_section, write_nx_ir_image, Cells as NxIrCells, NxIrImage, NxIrImageError,
    NxIrModuleRef, Table as NxIrTable, NONE as NX_IR_NONE, NX_IR_MAGIC,
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
mod ir_corpus_tests;
#[cfg(test)]
mod ir_tests;
#[cfg(test)]
mod tests;
