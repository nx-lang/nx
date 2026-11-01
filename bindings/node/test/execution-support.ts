/** Integration-only composition; the SDK deliberately exports no source execution helpers. */
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import { buildProgramArtifactFromSource, type NxProgramArtifact, type NxSourceBuildOptions, type NxSourceInput } from "../src/index.js";
export function evaluateIr(artifact: NxProgramArtifact) {
  const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image => [image.identity, prepareNxIrModule(image.bytes)]));
  return evaluateFunction(linkNxIrProgram(modules.get(artifact.entryIdentity)!, { resolve: identity => modules.get(identity) }), "root");
}
export function evaluateSource(source: NxSourceInput, options: NxSourceBuildOptions = {}) {
  const artifact = buildProgramArtifactFromSource(source, options);
  try { return evaluateIr(artifact); } finally { artifact.dispose(); }
}
