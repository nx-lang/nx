/**
 * Turns NX source into NX IR against the DrawnUI catalog, wherever the compiler is running.
 *
 * <para>The catalog is a module of its own, `skia.nx`, and the visitor's text is `playground.nx`;
 * the build names the catalog as an implicit import, so the visitor's document is analyzed as it
 * is and every diagnostic is already in its own coordinates. A compile emits the visitor's module
 * alone: its artifact names the catalog in its module table and carries none of the catalog's
 * declarations. The catalog's own artifact is emitted once, by `emitCatalogArtifact`, and the
 * renderer links every compile against that one preparation.</para>
 *
 * <para>What this module adds is the site's policy: the input limits, the classification of a
 * diagnostic by the module it belongs to, and the artifact handed to the renderer.</para>
 *
 * <para>Nothing here is browser-only or Node-only: the worker calls it with a host over the wasm
 * module, the Vite build calls it with a host over the same module under Node to bundle the
 * catalog's artifact, and `scripts/check-examples.mjs` does the same, so what the examples are
 * checked against is what the site ships.</para>
 */
import { NxEvaluationError, type NxHost } from "@nx-lang/sdk-wasm";

import type {
  CompileResult,
  CompiledArtifact,
  Diagnostic,
  DiagnosticOrigin,
  SdkDiagnostic,
} from "./types.ts";

const encoder = new TextEncoder();

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/** The logical file name the visitor's document compiles under. */
export const FILE_NAME = "playground.nx";

/** The logical identity of the catalog module. */
export const CATALOG_IDENTITY = "skia.nx";

/**
 * Compiles `source` against `catalog` through `host`.
 *
 * Returns `{ ir, diagnostics }`. `ir` is the visitor's module as an NX IR artifact, or null when
 * compilation failed; each diagnostic carries an `origin` saying whether it points at the visitor's
 * source or at the catalog behind it.
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

  try {
    const artifact = host.buildWorkspaceArtifact({
      modules: [
        { identity: CATALOG_IDENTITY, source: catalog },
        { identity: FILE_NAME, source },
      ],
      entry: FILE_NAME,
      implicitImports: [CATALOG_IDENTITY],
    });
    try {
      // The entry alone, without a debug section: a few kilobytes naming the catalog, which the
      // renderer already holds prepared.
      const [entry] = artifact.generateNxIr();
      return { ir: entry!.bytes, diagnostics: [] };
    } finally {
      artifact.dispose();
    }
  } catch (error) {
    if (error instanceof NxEvaluationError) {
      return { ir: null, diagnostics: error.diagnostics.map(classify) };
    }
    throw error;
  }
}

/**
 * Compiles the catalog on its own and returns its NX IR artifact.
 *
 * This is the artifact the site bundles at build time and prepares once per page. It is emitted
 * from the same catalog text every compile builds against, so a compiled snippet's module table
 * names exactly this module.
 *
 * @throws `NxEvaluationError` when the catalog itself does not compile — an application fault,
 * since the visitor cannot have caused it.
 */
export function emitCatalogArtifact(host: NxHost, catalog: string): CompiledArtifact {
  const artifact = host.buildWorkspaceArtifact({
    modules: [{ identity: CATALOG_IDENTITY, source: catalog }],
    entry: CATALOG_IDENTITY,
  });
  try {
    const [entry] = artifact.generateNxIr();
    return entry!.bytes;
  } finally {
    artifact.dispose();
  }
}

/** Classifies an SDK diagnostic by the module its primary label names. */
function classify(diagnostic: SdkDiagnostic): Diagnostic {
  const label = diagnostic.labels.find((candidate) => candidate.primary) ?? diagnostic.labels[0];
  const origin: DiagnosticOrigin =
    label === undefined || label.file === ""
      ? "program"
      : label.file === CATALOG_IDENTITY
        ? "catalog"
        : "source";
  return {
    severity: diagnostic.severity,
    ...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
    message: diagnostic.message,
    origin,
    span: origin === "source" && label !== undefined ? label.span : null,
  };
}
