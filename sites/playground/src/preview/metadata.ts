import type { NxHost } from "@nx-lang/sdk-wasm";
import type { SourceDeclaration } from "@nx-lang/language-protocol";
import { FILE_NAME, MODEL_URI } from "../compile/policy.ts";

/** Reads presentation metadata only; snapshot ownership ends even if the query fails. */
export function previewDeclarations(
  host: Pick<NxHost, "createLanguageSnapshot">,
  source: string,
): readonly SourceDeclaration[] {
  const snapshot = host.createLanguageSnapshot([
    { uri: MODEL_URI, identity: FILE_NAME, source },
  ]);
  try {
    return snapshot.sourceTree(MODEL_URI, {
      visualizers: { includeUnreferencedDeclarations: true },
    }).declarations;
  } finally {
    snapshot.dispose();
  }
}
