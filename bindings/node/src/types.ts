/**
 * NX source text or UTF-8 source bytes accepted by the Node SDK.
 */
export type NxSourceInput = string | Buffer | Uint8Array;

/**
 * Binary representation returned by byte-oriented evaluation APIs.
 */
export type NxOutputFormat = "messagePack" | "json";

/**
 * Severity level reported for an NX diagnostic.
 */
export type NxSeverity = "error" | "warning" | "info" | "hint";

/**
 * JSON-compatible value returned by NX JSON evaluation.
 */
export type NxJsonValue =
  | null
  | boolean
  | number
  | string
  | readonly NxJsonValue[]
  | { readonly [key: string]: NxJsonValue };

/**
 * Source module submitted as part of an in-memory NX workspace.
 */
export interface NxWorkspaceModuleInput {
  /**
   * Logical module identity used for imports, diagnostics, and workspace entry selection.
   *
   * Identities are normalized by NX logical path rules; callers do not need to create files on disk.
   */
  readonly identity: string;

  /**
   * NX source text or UTF-8 source bytes for this module.
   */
  readonly source: NxSourceInput;
}

/**
 * Half-open source span with byte offsets and 1-based line and column positions.
 */
export interface NxTextSpan {
  /**
   * Starting byte offset, inclusive.
   */
  readonly startByte: number;

  /**
   * Ending byte offset, exclusive.
   */
  readonly endByte: number;

  /**
   * Starting line number, using 1-based indexing.
   */
  readonly startLine: number;

  /**
   * Starting column number, using 1-based indexing.
   */
  readonly startColumn: number;

  /**
   * Ending line number, using 1-based indexing.
   */
  readonly endLine: number;

  /**
   * Ending column number, using 1-based indexing.
   */
  readonly endColumn: number;
}

/**
 * Source location attached to an NX diagnostic.
 */
export interface NxDiagnosticLabel {
  /**
   * Logical file or workspace identity for the labeled span.
   */
  readonly file: string;

  /**
   * Span of source associated with this label.
   */
  readonly span: NxTextSpan;

  /**
   * Optional message specific to this label.
   */
  readonly message?: string;

  /**
   * Whether this label marks the primary location for the diagnostic.
   */
  readonly primary: boolean;
}

/**
 * Structured diagnostic reported by NX validation, build, IR generation, or evaluation.
 */
export interface NxDiagnostic {
  /**
   * Diagnostic severity.
   */
  readonly severity: NxSeverity;

  /**
   * Stable diagnostic code when one is available.
   */
  readonly code?: string;

  /**
   * Primary diagnostic message.
   */
  readonly message: string;

  /**
   * Source labels related to this diagnostic.
   */
  readonly labels: readonly NxDiagnosticLabel[];

  /**
   * Optional help text with remediation guidance.
   */
  readonly help?: string;

  /**
   * Optional note with additional context.
   */
  readonly note?: string;
}

/**
 * Module-qualified declaration reference in generated NX IR metadata.
 */
export interface NxIrReferenceMetadata {
  /**
   * Stable IR module identifier.
   */
  readonly module: string;

  /**
   * Stable IR declaration identifier.
   */
  readonly declaration: string;

  /**
   * Authored NX declaration name.
   */
  readonly name: string;

  /**
   * Declaration kind, such as a function or component.
   */
  readonly kind: string;
}

/**
 * Public entrypoint metadata in a generated NX IR artifact.
 */
export interface NxIrEntrypointMetadata {
  /**
   * Public NX entrypoint name.
   */
  readonly name: string;

  /**
   * Resolved IR reference for the entrypoint.
   */
  readonly reference: NxIrReferenceMetadata;
}

/**
 * Structured metadata emitted alongside deterministic NX IR JSON.
 */
export interface NxIrMetadata {
  /**
   * Fingerprint of the analyzed NX program used for cache keys and equivalence checks.
   */
  readonly programFingerprint: number;

  /**
   * NX IR schema version.
   */
  readonly schemaVersion: number;

  /**
   * TypeScript IR runtime ABI required by this artifact.
   */
  readonly runtimeAbi: string;

  /**
   * Runtime feature flags required before loading this artifact.
   */
  readonly requiredFeatures: readonly string[];

  /**
   * Public function entrypoints emitted in the IR artifact.
   */
  readonly functionEntrypoints: readonly NxIrEntrypointMetadata[];

  /**
   * Public component entrypoints emitted in the IR artifact.
   */
  readonly componentEntrypoints: readonly NxIrEntrypointMetadata[];
}

/**
 * Deterministic NX IR JSON plus metadata generated from an NX program artifact.
 */
export interface NxGeneratedNxIr {
  /**
   * Deterministic NX IR JSON text exactly as emitted by the native generator.
   */
  readonly json: string;

  /**
   * Structured metadata for the generated IR document.
   */
  readonly metadata: NxIrMetadata;
}

/**
 * Options shared by evaluation APIs.
 */
export interface NxEvaluationOptions {
  /**
   * Entrypoint to evaluate.
   *
   * Only `root` is supported by the current native host API. Other values throw `NxEvaluationError` with an
   * `unsupported-entrypoint` diagnostic.
   */
  readonly entrypoint?: "root" | string;
}

/**
 * Options for byte-oriented evaluation APIs.
 */
export interface NxByteEvaluationOptions extends NxEvaluationOptions {
  /**
   * Requested output encoding. Defaults to `messagePack`.
   */
  readonly outputFormat?: NxOutputFormat;
}

/**
 * Options for building a program artifact from a single NX source payload.
 */
export interface NxSourceBuildOptions {
  /**
   * Build context used for resolving preloaded libraries.
   *
   * When omitted, the SDK creates and disposes a temporary empty context for the operation.
   */
  readonly buildContext?: import("./index.js").NxProgramBuildContext;

  /**
   * Logical file name used in diagnostics and local import normalization.
   */
  readonly fileName?: string;
}

/**
 * Options for evaluating a single NX source payload to JSON.
 */
export interface NxSourceEvaluationOptions extends NxSourceBuildOptions, NxEvaluationOptions {}

/**
 * Options for evaluating a single NX source payload to bytes.
 */
export interface NxSourceByteEvaluationOptions extends NxSourceBuildOptions, NxByteEvaluationOptions {}

/**
 * Options for building a program artifact from an in-memory workspace.
 */
export interface NxWorkspaceBuildOptions {
  /**
   * Build context used for resolving libraries that are not provided by workspace modules.
   */
  readonly buildContext: import("./index.js").NxProgramBuildContext;

  /**
   * Logical identity of the workspace module selected as the program entry module.
   */
  readonly entryIdentity: string;
}
