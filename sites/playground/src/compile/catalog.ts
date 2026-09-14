/**
 * Turns NX source into NX IR against the DrawnUI catalog, wherever the compiler is running.
 *
 * <para>The visitor's source and the catalog compile as a single module rather than through an
 * import, because an imported external component loses its defaults and its inherited props
 * (NXE12/NXE13).</para>
 *
 * <para>The catalog goes *first*. A source file may end in a single bare element expression instead
 * of declaring `root`, and the grammar allows that element only as the file's last item — appending
 * the catalog would put declarations after it and turn the whole file into a syntax error. Leading
 * with the catalog costs one subtraction per diagnostic: the visitor's text starts at a known line
 * and byte offset, and `classify` shifts spans back into the visitor's own coordinates. A
 * diagnostic that lands before that offset is inside the catalog, which is an application fault
 * rather than an authoring error. The shift itself is `@nx-lang/language-core`'s prelude
 * arithmetic, shared with the language service so the two cannot disagree about where the visitor's
 * text starts.</para>
 *
 * <para>Nothing here is browser-only or Node-only: the worker calls it with a host over the wasm
 * module, and `scripts/check-examples.mjs` calls it with a host over the same module under Node, so
 * what the examples are checked against is what the site ships.</para>
 */
import { preludeOffsets, withPrelude, type PreludeOffsets } from "@nx-lang/language-core";
import type {
  NxDiagnostic,
  NxDiagnosticLabel,
  NxHost,
  NxProgramArtifact,
  NxTextSpan,
} from "@nx-lang/sdk-wasm";

import type { CompileResult, Diagnostic, DiagnosticSpan } from "./types.ts";

const encoder = new TextEncoder();

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/** The logical file name the combined module compiles under. */
const FILE_NAME = "playground.nx";

/**
 * The location a whole-program failure carries in place of one: an empty span at the very first
 * character of the combined module.
 *
 * That character belongs to the catalog, so a diagnostic genuinely reported there is an application
 * fault as well, and reporting it the same way costs nothing.
 */
function isProgramSentinel(span: NxTextSpan): boolean {
  return span.startByte === 0 && span.endByte === 0 && span.startLine === 1 && span.startColumn === 1;
}

/**
 * A label points somewhere unless it carries that sentinel instead of a location.
 *
 * Width is not the test. A compiler names an insertion point with an empty span — `Expected } here`
 * arrives with one, at the exact column the missing token belongs before — and discarding those
 * would throw away the only position the author can act on.
 */
function isPositioned(label: NxDiagnosticLabel): boolean {
  return label.span !== undefined && label.span !== null && !isProgramSentinel(label.span);
}

/**
 * Where a diagnostic belongs: `source`, `catalog`, or `program`.
 *
 * A diagnostic is the visitor's as soon as any of its labels lands in their source — a duplicate
 * declaration points at both their line and the catalog's, and theirs is the one they can act on.
 * `catalog` is an application fault: the visitor cannot have caused it and must not be shown a
 * marker for it. `program` is for whole-program failures, which carry an empty span at 1:1 rather
 * than a location; marking those on the visitor's first line would be a lie about where the problem
 * is, so they are reported without a position.
 */
function classify(diagnostic: NxDiagnostic, prefix: PreludeOffsets): Diagnostic {
  const positioned = (diagnostic.labels ?? []).filter(isPositioned);
  const inVisitor = positioned.find((label) => label.span.startLine > prefix.lines);
  const origin = inVisitor !== undefined ? "source" : positioned.length > 0 ? "catalog" : "program";
  return {
    severity: diagnostic.severity,
    code: diagnostic.code ?? "",
    message: diagnostic.message,
    origin,
    span: inVisitor === undefined ? null : toVisitorSpan(inVisitor.span, prefix),
  };
}

/**
 * Shifts a span from combined-module coordinates back into the visitor's own.
 *
 * Only lines and bytes move; a column is relative to its line's start, and the catalog contributes
 * whole lines, so columns carry over untouched.
 */
function toVisitorSpan(span: NxTextSpan, prefix: PreludeOffsets): DiagnosticSpan {
  return {
    startByte: span.startByte - prefix.bytes,
    endByte: span.endByte - prefix.bytes,
    startLine: span.startLine - prefix.lines,
    startColumn: span.startColumn,
    endLine: span.endLine - prefix.lines,
    endColumn: span.endColumn,
  };
}

/**
 * Compiles `source` against `catalog` through `host`.
 *
 * Returns `{ ir, diagnostics }`. `ir` is null when compilation failed; each diagnostic carries an
 * `origin` saying whether it points at the visitor's source or at the catalog behind it.
 *
 * @throws TypeError when `source` is not a string, RangeError when it is over the size limit, and
 * `NxHostCrashedError` when the module traps — a crash is the caller's to recover from by replacing
 * the host, not something to report as an authoring error.
 */
export function compileWithCatalog(host: NxHost, catalog: string, source: string): CompileResult {
  if (typeof source !== "string") {
    throw new TypeError("source must be a string");
  }
  if (encoder.encode(source).byteLength > MAX_SOURCE_BYTES) {
    throw new RangeError(`source exceeds ${MAX_SOURCE_BYTES} bytes`);
  }

  // The same shift the language service applies to the same catalog, so a diagnostic from a compile
  // and a range from a hover land on the same visitor line.
  const prefix = preludeOffsets(catalog);
  const combined = withPrelude(prefix, source);

  let artifact;
  try {
    artifact = host.buildProgramArtifact(combined, { fileName: FILE_NAME });
  } catch (error) {
    return { ir: null, diagnostics: diagnosticsOf(error, prefix) };
  }

  try {
    const ir = JSON.parse(artifact.generateNxIr().json) as unknown;
    release(artifact);
    return { ir, diagnostics: [] };
  } catch (error) {
    release(artifact);
    return { ir: null, diagnostics: diagnosticsOf(error, prefix) };
  }
}

/**
 * Releases the artifact without letting the release decide what the compile returns.
 *
 * A trap inside `dispose` would otherwise replace a result the caller already has with a crash. The
 * host remembers that it crashed, so the next call reports it and the worker replaces the host;
 * losing the IR that was already produced buys nothing.
 */
function release(artifact: NxProgramArtifact): void {
  try {
    artifact.dispose();
  } catch {
    // Deliberately swallowed: see above.
  }
}

/** The classified diagnostics an NX failure carries, or a rethrow when it carries none. */
function diagnosticsOf(error: unknown, prefix: PreludeOffsets): readonly Diagnostic[] {
  const diagnostics = (error as { diagnostics?: unknown } | null | undefined)?.diagnostics;
  if (!Array.isArray(diagnostics)) {
    throw error;
  }
  return diagnostics.map((diagnostic: NxDiagnostic) => classify(diagnostic, prefix));
}
