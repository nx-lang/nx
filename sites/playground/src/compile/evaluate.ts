/**
 * Evaluates NX source to annotated NX text, wherever the compiler is running.
 *
 * <para>What this module adds to `evaluateNx()` is the site's policy: the input limit, the output
 * limit, and the classification of a diagnostic by where it points. Nothing here is browser-only or
 * Node-only: the worker calls it with a host over the wasm module, and
 * `scripts/check-examples.mjs` calls it with a host over the same module under Node, so what the
 * examples are checked against is what the site ships.</para>
 */
import { NxEvaluationError, type NxHost, type NxValueText } from "@nx-lang/sdk-wasm";

import type { Diagnostic, DiagnosticOrigin, EvaluateResult, Outcome, SdkDiagnostic } from "./types.ts";

const encoder = new TextEncoder();

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/**
 * Output longer than this is cut before it leaves the worker, so an enormous value is never cloned
 * to the page or laid out there.
 */
export const MAX_OUTPUT_CHARACTERS = 100_000;

/** The logical file name the visitor's document compiles under. */
export const FILE_NAME = "playground.nx";

/**
 * Compiles `source` and evaluates its `root` through `host`.
 *
 * @throws TypeError when `source` is not a string, RangeError when it is over the size limit, and
 * `NxHostCrashedError` when the module traps — a crash is the caller's to recover from by replacing
 * the host, not something to report as an authoring error.
 */
export function evaluateSource(host: NxHost, source: string): EvaluateResult {
  if (typeof source !== "string") {
    throw new TypeError("source must be a string");
  }
  if (encoder.encode(source).byteLength > MAX_SOURCE_BYTES) {
    throw new RangeError(`source exceeds ${MAX_SOURCE_BYTES} bytes`);
  }

  let artifact;
  try {
    artifact = host.buildProgramArtifact(source, { fileName: FILE_NAME });
  } catch (error) {
    if (error instanceof NxEvaluationError) {
      return { diagnostics: error.diagnostics.map(classifyDiagnostic), outcome: null };
    }
    throw error;
  }

  try {
    return { diagnostics: [], outcome: valueOutcome(artifact.evaluateNx()) };
  } catch (error) {
    if (!(error instanceof NxEvaluationError)) {
      throw error;
    }
    const diagnostics = error.diagnostics.map(classifyDiagnostic);
    const outcome: Outcome = diagnostics.some((diagnostic) => diagnostic.code === "no-root")
      ? { kind: "noRoot" }
      : { kind: "error", diagnostics };
    return { diagnostics: [], outcome };
  } finally {
    artifact.dispose();
  }
}

/** The value as the page receives it: cut to the output limit, with only the nodes that fit. */
export function valueOutcome(value: NxValueText): Outcome {
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
