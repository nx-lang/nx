import type { NxDiagnostic, NxSeverity, NxTextSpan } from "@nx-lang/sdk-wasm";

/**
 * Where a diagnostic points, and therefore how the app is allowed to present it.
 *
 * - `source`: the visitor's document. The diagnostic carries a span in its own coordinates.
 * - `catalog`: the DrawnUI catalog module. The visitor cannot have caused it.
 * - `program`: the program as a whole, with no location.
 */
export type DiagnosticOrigin = "source" | "catalog" | "program";

export type DiagnosticSpan = NxTextSpan;

/**
 * `source` diagnostics carry a span in the author's own coordinates and are marked in the editor.
 * `catalog` and `program` diagnostics are application faults, reported without a position.
 */
export interface Diagnostic {
  readonly severity: NxSeverity;
  /** The stable diagnostic code, when NX reports one. */
  readonly code?: string;
  readonly message: string;
  readonly origin: DiagnosticOrigin;
  readonly span: DiagnosticSpan | null;
}

/**
 * One module's NX IR artifact: the image, as bytes.
 *
 * Opaque on this side of the seam: the renderer prepares it with the IR runtime, which owns the
 * layout. A compiled snippet's artifact names the catalog in its module table and carries none of
 * the catalog's declarations; the catalog's own artifact is bundled at build time. The bytes are
 * the artifact's own buffer, so a worker transfers them rather than copying.
 */
export type CompiledArtifact = Uint8Array;

export interface CompileResult {
  /** The visitor's module as an NX IR artifact, or null when compilation failed. */
  readonly ir: CompiledArtifact | null;
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

/** The raw diagnostic shape the SDK reports, re-exported for the classifier. */
export type SdkDiagnostic = NxDiagnostic;
