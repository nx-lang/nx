import type { NxDiagnostic, NxSeverity, NxTextSpan, NxValueText } from "@nx-lang/sdk-wasm";

/**
 * Where a diagnostic points, and therefore how the app is allowed to present it.
 *
 * - `source`: the visitor's document. The diagnostic carries a span in its own coordinates.
 * - `program`: the program as a whole, or the compiler's own prelude, with no position the visitor
 *   can be shown.
 */
export type DiagnosticOrigin = "source" | "program";

export type DiagnosticSpan = NxTextSpan;

/**
 * `source` diagnostics carry a span in the visitor's own coordinates and are marked in the editor.
 * `program` diagnostics are reported without a position.
 */
export interface Diagnostic {
  readonly severity: NxSeverity;
  /** The stable diagnostic code, when NX reports one. */
  readonly code?: string;
  readonly message: string;
  readonly origin: DiagnosticOrigin;
  readonly span: DiagnosticSpan | null;
}

/** What evaluating a program that compiled came to. */
export type Outcome =
  /** `root`'s value as annotated NX text, cut to the output limit when `truncated`. */
  | { readonly kind: "value"; readonly value: NxValueText; readonly truncated: boolean }
  /** The program compiled but declares no `root` and ends in no element. */
  | { readonly kind: "noRoot" }
  /** Evaluating `root` failed at run time, or its value has no NX spelling. */
  | { readonly kind: "error"; readonly diagnostics: readonly Diagnostic[] };

export interface EvaluateResult {
  /** What compiling reported. Non-empty means the program did not compile. */
  readonly diagnostics: readonly Diagnostic[];
  /** What evaluating came to, or null when the program did not compile. */
  readonly outcome: Outcome | null;
}

/**
 * The one seam between authoring and evaluation.
 *
 * Evaluation happens in the browser, in a worker over the WebAssembly build of the compiler.
 * Everything upstream of this interface — the editor and the output pane — is written against the
 * interface alone and knows nothing of where the compiler runs.
 */
export type Evaluate = (source: string) => Promise<EvaluateResult>;

/** The raw diagnostic shape the SDK reports, re-exported for the classifier. */
export type SdkDiagnostic = NxDiagnostic;
