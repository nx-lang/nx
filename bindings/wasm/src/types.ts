/**
 * Severity level reported for an NX diagnostic.
 */
export type NxSeverity = "error" | "warning" | "info" | "hint";

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
 * Structured diagnostic reported by NX build or IR generation.
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
  readonly programFingerprint: string;

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
   * Deterministic NX IR JSON text exactly as emitted by the generator.
   */
  readonly json: string;

  /**
   * Structured metadata for the generated IR document.
   */
  readonly metadata: NxIrMetadata;
}

/**
 * Options for building a program artifact from a single NX source text.
 */
export interface NxSourceBuildOptions {
  /**
   * Logical file name used in diagnostics and local import normalization. Defaults to `input.nx`.
   */
  readonly fileName?: string;
}

/**
 * One in-memory document submitted to a language snapshot.
 */
export interface NxLanguageDocumentInput {
  /**
   * Logical URI the document is addressed by in queries, such as `nx://tenant/form.nx`.
   */
  readonly uri: string;

  /**
   * Complete source text.
   */
  readonly source: string;

  /**
   * Workspace identity NX resolves imports by, such as `tenant/form.nx`. Derived from the URI when omitted.
   */
  readonly identity?: string;

  /**
   * Editor version of the text, echoed back in every result for that document.
   */
  readonly version?: number;
}
