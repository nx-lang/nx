// Everything both entry points expose. Only `createNxHost` differs between them, because only the
// WASI provider differs.

export { abiVersion } from "./abi.js";
export {
  NxDisposedResourceError,
  NxEvaluationError,
  NxHostCrashedError,
  NxWasmError
} from "./errors.js";
export { createLanguageService, type NxLanguageServiceOptions } from "./language.js";
export { compileNxModule, type NxModuleSource } from "./module.js";
export {
  buildProgramWithPrelude,
  type NxDiagnosticOrigin,
  type NxPreludeBuildOptions,
  type NxPreludeBuildResult,
  type NxPreludeDiagnostic
} from "./prelude.js";
export type { NxHost, NxLanguageSnapshot, NxProgramArtifact } from "./host.js";
export type {
  SnapshotLanguageService,
  SnapshotLike,
  PreludeOffsets
} from "@nx-lang/language-core";
export type {
  NxDiagnostic,
  NxDiagnosticLabel,
  NxGeneratedNxIr,
  NxIrEntrypointMetadata,
  NxIrMetadata,
  NxIrReferenceMetadata,
  NxLanguageDocumentInput,
  NxSeverity,
  NxSourceBuildOptions,
  NxTextSpan
} from "./types.js";
export type {
  CompletionItem,
  CompletionItemKind,
  CompletionList,
  DiagnosticReport,
  DiagnosticSeverity,
  DocumentDiagnostics,
  DocumentSymbol,
  DocumentSymbolKind,
  EditorDiagnostic,
  EditorRange,
  Hover,
  RelatedLocation,
  TextPosition,
  WorkspaceDiagnostic,
  WorkspaceDiagnosticLabel
} from "@nx-lang/language-protocol";
