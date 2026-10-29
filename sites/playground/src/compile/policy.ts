/** Shared source limits, output formatting limits, and diagnostic classification for the IR workbench. */
import type { NxValueText } from "@nx-lang/sdk-wasm";
import type { Diagnostic, DiagnosticOrigin, Outcome, SdkDiagnostic } from "./types.ts";

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/**
 * Output longer than this is cut before it leaves the worker, so an enormous value is never cloned
 * to the page or laid out there.
 */
export const MAX_OUTPUT_CHARACTERS = 100_000;

/** The logical file name the visitor's document compiles under. */
export const FILE_NAME = "playground.nx";
/** Logical editor URI shared by the page and worker snapshots. */
export const MODEL_URI = "nx://playground/playground.nx";

/** The value as the page receives it: cut to the output limit, with only the nodes that fit. */
export function valueOutcome(value: NxValueText): Extract<Outcome, { kind: "value" }> {
  if (value.text.length <= MAX_OUTPUT_CHARACTERS) {
    return { kind: "value", value, truncated: false };
  }
  const cut = cutPoint(value.text);
  const nodes = value.nodes
    .filter((node) => node.start < cut)
    .map((node) => (node.end > cut ? { ...node, end: cut } : node));
  return { kind: "value", value: { text: value.text.slice(0, cut), nodes }, truncated: true };
}

/**
 * Where to cut text longer than the limit: after the last whole line within it, so the notice
 * follows complete lines, or, for one long line, at the limit moved back off a surrogate pair's
 * first half, so no character is split.
 */
export function cutPoint(text: string): number {
  const lineEnd = text.lastIndexOf("\n", MAX_OUTPUT_CHARACTERS - 1);
  if (lineEnd > 0) {
    return lineEnd;
  }
  const code = text.charCodeAt(MAX_OUTPUT_CHARACTERS - 1);
  return code >= 0xd800 && code <= 0xdbff ? MAX_OUTPUT_CHARACTERS - 1 : MAX_OUTPUT_CHARACTERS;
}

/** Classifies an SDK diagnostic by the file its primary label names. */
export function classifyDiagnostic(diagnostic: SdkDiagnostic): Diagnostic {
  const label = diagnostic.labels.find((candidate) => candidate.primary) ?? diagnostic.labels[0];
  // A label anywhere but the visitor's own file — the prelude, say — is the compiler's own text,
  // not a position in the editor.
  const origin: DiagnosticOrigin = label !== undefined && label.file === FILE_NAME ? "source" : "program";
  return {
    severity: diagnostic.severity,
    ...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
    message: diagnostic.message,
    origin,
    span: origin === "source" && label !== undefined ? label.span : null
  };
}
