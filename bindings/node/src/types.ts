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
 * JSON-compatible value returned by NX JSON evaluation, in NX's canonical encoding.
 *
 * NX has no `null` value. The empty value (`{}` in NX source, a `?` or `*` occurrence that holds
 * nothing) encodes as `[]` outside a record, and an empty optional record field is an omitted key. A
 * `+` or `*` value is an array; a `?` value that holds an item is the item itself.
 */
export type NxJsonValue = boolean | number | string | readonly NxJsonValue[] | NxJsonRecord;

/**
 * A record in NX's canonical JSON encoding: its `$type` and the fields it stores.
 *
 * An ordinary record never stores an empty field, so its keys are all present values. An update
 * record (a `$type` ending in `.Update`) is the one place `null` appears: a key present with `null`
 * means that field is cleared, while an absent key means it is unchanged.
 */
export interface NxJsonRecord {
  readonly [key: string]: NxJsonValue | null;
}

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

  /**
   * The version string every artifact built from this workspace records for this module in its
   * module table, so a runtime linking against a prepared module can tell whether it is the one the
   * artifact was built against. NX never reads it. Records `""` when omitted.
   */
  readonly version?: string;
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
   * Whether each artifact carries its debug section: spans and source text. Off by default.
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
   * The artifact as an NX IR image, byte for byte what `@nx-lang/sdk-wasm` emits for the same
   * input. `@nx-lang/ir-runtime` reads it in place.
   */
  readonly bytes: Buffer;

  /**
   * Structured metadata for the generated artifact.
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
 * Options shared by every operation over an in-memory workspace.
 */
export interface NxWorkspaceOptions {
  /**
   * Workspace identities every other module imports implicitly, as if it began with a wildcard
   * import of each. A listed module imports nothing implicitly itself. An identity the workspace
   * does not hold fails the operation with a diagnostic naming it.
   */
  readonly implicitImports?: readonly string[];
}

/**
 * Options for building a program artifact from an in-memory workspace.
 */
export interface NxWorkspaceBuildOptions extends NxWorkspaceOptions {
  /**
   * Build context used for resolving libraries that are not provided by workspace modules.
   */
  readonly buildContext: import("./index.js").NxProgramBuildContext;

  /**
   * Logical identity of the workspace module selected as the program entry module.
   */
  readonly entryIdentity: string;
}
