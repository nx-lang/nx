/** Where a diagnostic points, and therefore how the app is allowed to present it. */
export type DiagnosticOrigin = "source" | "catalog" | "program";

export interface DiagnosticSpan {
  readonly startByte: number;
  readonly endByte: number;
  readonly startLine: number;
  readonly startColumn: number;
  readonly endLine: number;
  readonly endColumn: number;
}

export interface Diagnostic {
  readonly severity: string;
  readonly code: string;
  readonly message: string;
  /**
   * `source` diagnostics carry a span in the author's own coordinates and are marked in the editor.
   * `catalog` and `program` diagnostics are application faults, reported without a position.
   */
  readonly origin: DiagnosticOrigin;
  readonly span: DiagnosticSpan | null;
}

export interface CompileResult {
  /** The NX IR program, or null when compilation failed. */
  readonly ir: unknown | null;
  readonly diagnostics: readonly Diagnostic[];
}

/**
 * The one seam between authoring and compilation.
 *
 * Compilation happens in the browser, in a worker over the WebAssembly build of the compiler.
 * Everything upstream of this interface — the editor, the renderer, the gallery — is written
 * against the interface alone and knows nothing of where the compiler runs.
 */
export type Compile = (source: string) => Promise<CompileResult>;
