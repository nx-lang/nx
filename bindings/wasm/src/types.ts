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
 * Structured metadata emitted alongside an NX IR artifact.
 */
export interface NxIrMetadata {
  /**
   * Workspace identity of the module the artifact carries.
   */
  readonly identity: string;

  /**
   * Fingerprint of the module's source text, as a decimal string so it compares without `number`
   * precision loss. It is also the first module-table entry's fingerprint in the artifact.
   */
  readonly fingerprint: string;

  /**
   * NX IR schema version.
   */
  readonly schemaVersion: number;

  /**
   * IR runtime ABI required by this artifact.
   */
  readonly runtimeAbi: string;

  /**
   * Runtime feature flags required before loading this artifact.
   */
  readonly requiredFeatures: readonly string[];

  /**
   * Names of the module's top-level functions, in declaration order.
   */
  readonly functionEntrypoints: readonly string[];

  /**
   * Names of the module's top-level components, in declaration order.
   */
  readonly componentEntrypoints: readonly string[];
}

/**
 * What to emit from a program artifact.
 */
export interface NxIrEmitOptions {
  /**
   * Identities of the modules to emit an artifact for. Omitted, the entry module alone is emitted;
   * an empty list emits every module of the program, entry first.
   */
  readonly modules?: readonly string[];

  /**
   * Whether each artifact carries its debug section: spans and source text. Off by default, since
   * the IR travels inside shares where nobody reads it.
   */
  readonly debug?: boolean;
}

/**
 * One NX IR artifact generated from an NX program artifact: its image and its metadata.
 */
export interface NxGeneratedNxIr {
  /**
   * Workspace identity of the module the artifact carries.
   */
  readonly identity: string;

  /**
   * The artifact as an NX IR image, copied out of the module's memory. The bytes are the caller's:
   * they outlive the artifact and the host, and `@nx-lang/ir-runtime` reads them in place.
   */
  readonly bytes: Uint8Array;

  /**
   * Structured metadata for the generated artifact.
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
 * One module of an in-memory workspace.
 */
export interface NxWorkspaceModuleInput {
  /**
   * Workspace identity NX resolves imports by, such as `drawnui.nx` or `app/main.nx`.
   */
  readonly identity: string;

  /**
   * Complete source text.
   */
  readonly source: string;

  /**
   * The version string every artifact built from this workspace records for this module in its
   * module table, so a runtime linking against a prepared module can tell whether it is the one
   * the artifact was built against. NX never reads it. Records `""` when omitted.
   */
  readonly version?: string;
}

/**
 * Options for building a program artifact from a workspace of in-memory modules.
 */
export interface NxWorkspaceBuildOptions {
  readonly modules: readonly NxWorkspaceModuleInput[];

  /**
   * Identity of the module the program is built for.
   */
  readonly entry: string;

  /**
   * Identities every other module imports implicitly, as if it began with a wildcard import of
   * each. A listed module imports nothing implicitly itself. An identity the workspace does not
   * hold fails the build with a diagnostic naming it.
   */
  readonly implicitImports?: readonly string[];
}

/**
 * Options for analyzing documents into a language snapshot.
 */
export interface NxLanguageSnapshotOptions {
  /**
   * Identities every document imports implicitly; see `NxWorkspaceBuildOptions.implicitImports`.
   */
  readonly implicitImports?: readonly string[];
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
