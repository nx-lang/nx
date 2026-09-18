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
export type { NxHost, NxLanguageSnapshot, NxProgramArtifact } from "./host.js";
export type { SnapshotLanguageService, SnapshotLike } from "@nx-lang/language-core";
export type {
  NxDiagnostic,
  NxDiagnosticLabel,
  NxGeneratedNxIr,
  NxIrEmitOptions,
  NxIrMetadata,
  NxLanguageDocumentInput,
  NxLanguageSnapshotOptions,
  NxSeverity,
  NxSourceBuildOptions,
  NxTextSpan,
  NxWorkspaceBuildOptions,
  NxWorkspaceModuleInput
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
