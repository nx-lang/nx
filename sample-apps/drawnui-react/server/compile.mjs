/**
 * Turns NX source into NX IR, in process, through the native SDK.
 *
 * The visitor's source and the catalog compile as a single module rather than through an import,
 * because an imported external component loses its defaults and its inherited props (NXE12/NXE13).
 *
 * The catalog goes *first*. A source file may end in a single bare element expression instead of
 * declaring `root`, and the grammar allows that element only as the file's last item — appending the
 * catalog would put declarations after it and turn the whole file into a syntax error. Leading with
 * the catalog costs one subtraction per diagnostic: the visitor's text starts at a known line and
 * byte offset, and `classify` shifts spans back into the visitor's own coordinates. A diagnostic
 * that lands before that offset is inside the catalog, which is an application fault rather than an
 * authoring error.
 */
import { NxProgramArtifact } from "@nx-lang/sdk-node";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const catalog = readFileSync(join(appRoot, "catalog/skia.nx"), "utf8");

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

function countLines(text) {
  let lines = 1;
  for (const character of text) {
    if (character === "\n") {
      lines += 1;
    }
  }
  return lines;
}

/**
 * The location a whole-program failure carries in place of one: an empty span at the very first
 * character of the combined module.
 *
 * That character belongs to the catalog, so a diagnostic genuinely reported there is an application
 * fault as well, and reporting it the same way costs nothing.
 */
function isProgramSentinel(span) {
  return (
    span.startByte === 0 && span.endByte === 0 && span.startLine === 1 && span.startColumn === 1
  );
}

/**
 * A label points somewhere unless it carries that sentinel instead of a location.
 *
 * Width is not the test. A compiler names an insertion point with an empty span — `Expected } here`
 * arrives with one, at the exact column the missing token belongs before — and discarding those
 * would throw away the only position the author can act on.
 */
function isPositioned(label) {
  const span = label?.span;
  return span !== undefined && span !== null && !isProgramSentinel(span);
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
function classify(diagnostic, prefix) {
  const labels = diagnostic.labels ?? [];
  const positioned = labels.filter(isPositioned);
  const inVisitor = positioned.find((label) => label.span.startLine > prefix.lines);
  const origin = inVisitor !== undefined ? "source" : positioned.length > 0 ? "catalog" : "program";
  return {
    severity: diagnostic.severity,
    code: diagnostic.code,
    message: diagnostic.message,
    origin,
    span: origin === "source" ? toVisitorSpan(inVisitor.span, prefix) : null,
  };
}

/**
 * Shifts a span from combined-module coordinates back into the visitor's own.
 *
 * Only lines and bytes move; a column is relative to its line's start, and the catalog contributes
 * whole lines, so columns carry over untouched.
 */
function toVisitorSpan(span, prefix) {
  return {
    ...span,
    startByte: span.startByte - prefix.bytes,
    endByte: span.endByte - prefix.bytes,
    startLine: span.startLine - prefix.lines,
    endLine: span.endLine - prefix.lines,
  };
}

/**
 * Compiles NX source against the catalog.
 *
 * Returns `{ ir, diagnostics }`. `ir` is null when compilation failed; diagnostics may be present
 * either way, and each carries an `origin` saying whether it points at the visitor's source or at
 * the catalog behind it.
 */
export function compile(source) {
  return compileWithCatalog(catalog, source);
}

/**
 * Compiles against a supplied catalog rather than the app's own.
 *
 * Exists so a test can prove the catalog-origin branch with a real diagnostic from a real broken
 * catalog, which is otherwise unreachable: the committed catalog compiles.
 */
export function compileWithCatalog(catalogSource, source) {
  if (typeof source !== "string") {
    throw new TypeError("source must be a string");
  }
  if (Buffer.byteLength(source, "utf8") > MAX_SOURCE_BYTES) {
    throw new RangeError(`source exceeds ${MAX_SOURCE_BYTES} bytes`);
  }

  const catalogText = catalogSource.endsWith("\n") ? catalogSource : `${catalogSource}\n`;
  const prefixText = `${catalogText}\n`;
  const prefix = {
    lines: countLines(prefixText) - 1,
    bytes: Buffer.byteLength(prefixText, "utf8"),
  };
  const combined = `${prefixText}${source}`;

  try {
    const artifact = NxProgramArtifact.buildSource(combined, { fileName: "fiddle.nx" });
    try {
      const generated = artifact.generateNxIr();
      return { ir: JSON.parse(generated.json), diagnostics: [] };
    } finally {
      artifact.dispose?.();
    }
  } catch (error) {
    const diagnostics = error?.diagnostics;
    if (!Array.isArray(diagnostics)) {
      throw error;
    }
    return { ir: null, diagnostics: diagnostics.map((item) => classify(item, prefix)) };
  }
}
