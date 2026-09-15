/**
 * Building a program from a prelude and a document, with every diagnostic told apart by where it
 * lands.
 *
 * <para>A host that keeps its context declarations — a catalog of external components, say — in a
 * prelude compiles the prelude and the document as a single module rather than through an import,
 * because an imported external component loses its defaults and its inherited props (NXE12/NXE13).
 * The prelude goes *first*: a document may end in a single bare element expression instead of
 * declaring `root`, and the grammar allows that element only as the file's last item, so appending
 * the prelude would turn the whole file into a syntax error.</para>
 *
 * <para>Leading with the prelude costs one subtraction per diagnostic: the document's text starts
 * at a known line and byte offset, and `classify` shifts spans back into the document's own
 * coordinates. A diagnostic that lands before that offset is inside the prelude, which is the
 * host's fault rather than the author's. The shift is `@nx-lang/language-core`'s prelude
 * arithmetic, the same one the language service applies, so a diagnostic from a build and a range
 * from a hover land on the same document line.</para>
 */
import { preludeOffsets, withPrelude, type PreludeOffsets } from "@nx-lang/language-core";

import type { NxHost, NxProgramArtifact } from "./host.js";
import type {
  NxDiagnostic,
  NxDiagnosticLabel,
  NxGeneratedNxIr,
  NxSeverity,
  NxTextSpan
} from "./types.js";

/**
 * Where a diagnostic from a prelude build belongs, and therefore how a host may present it.
 *
 * - `source`: the document. The diagnostic carries a span in the document's own coordinates and is
 *   the author's to act on.
 * - `catalog`: the prelude. The author cannot have caused it and must not be shown a marker for it.
 * - `program`: the program as a whole. Such a failure carries no location at all.
 */
export type NxDiagnosticOrigin = "source" | "catalog" | "program";

/** One diagnostic from a prelude build, classified by origin. */
export interface NxPreludeDiagnostic {
  readonly severity: NxSeverity;

  /** The stable diagnostic code, when NX reports one. */
  readonly code?: string;

  readonly message: string;

  readonly origin: NxDiagnosticOrigin;

  /**
   * The span in the document's own coordinates for a `source` diagnostic; `null` for `catalog`
   * and `program` diagnostics, which have no document position.
   */
  readonly span: NxTextSpan | null;
}

/** What a prelude build produces. */
export interface NxPreludeBuildResult {
  /** The generated NX IR, or `null` when the build failed. */
  readonly ir: NxGeneratedNxIr | null;

  /**
   * Empty when `ir` is present: the host reports NX diagnostics by throwing, so a build that
   * returned IR reported nothing at all. A successful build therefore carries no warnings either.
   */
  readonly diagnostics: readonly NxPreludeDiagnostic[];
}

/** Options for a prelude build. */
export interface NxPreludeBuildOptions {
  /**
   * Logical file name the combined module compiles under, used in diagnostics. Defaults to
   * `input.nx`.
   */
  readonly fileName?: string;
}

/**
 * Compiles `source` behind `prelude` through `host` and emits NX IR.
 *
 * <para>Returns `{ ir, diagnostics }`. `ir` is `null` when the build failed; each diagnostic then
 * carries an `origin` saying whether it points at the document, at the prelude behind it, or at
 * the program as a whole, with `source` spans shifted into the document's own coordinates. The
 * document is never blamed for a fault in the prelude.</para>
 *
 * @throws NxHostCrashedError when the module traps — a crash is the caller's to recover from by
 * replacing the host, not something to report as an authoring error.
 */
export function buildProgramWithPrelude(
  host: NxHost,
  prelude: string,
  source: string,
  options: NxPreludeBuildOptions = {}
): NxPreludeBuildResult {
  const offsets = preludeOffsets(prelude);
  const combined = withPrelude(offsets, source);
  const buildOptions = options.fileName === undefined ? {} : { fileName: options.fileName };

  let artifact: NxProgramArtifact;
  try {
    artifact = host.buildProgramArtifact(combined, buildOptions);
  } catch (error) {
    return { ir: null, diagnostics: diagnosticsOf(error, offsets) };
  }

  try {
    const ir = artifact.generateNxIr();
    release(artifact);
    return { ir, diagnostics: [] };
  } catch (error) {
    release(artifact);
    return { ir: null, diagnostics: diagnosticsOf(error, offsets) };
  }
}

/**
 * The location a whole-program failure carries in place of one: an empty span at the very first
 * character of the combined module.
 *
 * That character belongs to the prelude, so a diagnostic genuinely reported there is the host's
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
 * Classifies one diagnostic.
 *
 * A diagnostic is the document's as soon as any of its labels lands there — a duplicate declaration
 * points at both the author's line and the prelude's, and the author's is the one they can act on.
 */
function classify(diagnostic: NxDiagnostic, offsets: PreludeOffsets): NxPreludeDiagnostic {
  const positioned = (diagnostic.labels ?? []).filter(isPositioned);
  const inDocument = positioned.find((label) => label.span.startLine > offsets.lines);
  const origin: NxDiagnosticOrigin =
    inDocument !== undefined ? "source" : positioned.length > 0 ? "catalog" : "program";
  return {
    severity: diagnostic.severity,
    ...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
    message: diagnostic.message,
    origin,
    span: inDocument === undefined ? null : toDocumentSpan(inDocument.span, offsets)
  };
}

/**
 * Shifts a span from combined-module coordinates back into the document's own.
 *
 * Only lines and bytes move; a column is relative to its line's start, and the prelude contributes
 * whole lines, so columns carry over untouched.
 */
function toDocumentSpan(span: NxTextSpan, offsets: PreludeOffsets): NxTextSpan {
  return {
    startByte: span.startByte - offsets.bytes,
    endByte: span.endByte - offsets.bytes,
    startLine: span.startLine - offsets.lines,
    startColumn: span.startColumn,
    endLine: span.endLine - offsets.lines,
    endColumn: span.endColumn
  };
}

/**
 * Releases the artifact without letting the release decide what the build returns.
 *
 * A trap inside `dispose` would otherwise replace a result the caller already has with a crash. The
 * host remembers that it crashed, so the next call reports it and the caller replaces the host;
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
function diagnosticsOf(error: unknown, offsets: PreludeOffsets): readonly NxPreludeDiagnostic[] {
  const diagnostics = (error as { diagnostics?: unknown } | null | undefined)?.diagnostics;
  if (!Array.isArray(diagnostics)) {
    throw error;
  }
  return diagnostics.map((diagnostic: NxDiagnostic) => classify(diagnostic, offsets));
}
