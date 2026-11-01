/** Integration-only IR execution followed by the independent canonical-value formatter. */
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import type { NxProgramArtifact } from "../src/host.js";
export function formatIrResult(artifact: NxProgramArtifact, options: { type?: string } = {}) {
  const entry = artifact.generateNxIr()[0]!;
  const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image => [image.identity, prepareNxIrModule(image.bytes)]));
  const program = linkNxIrProgram(modules.get(entry.identity)!, { resolve: identity => modules.get(identity) });
  return artifact.formatValue(evaluateFunction(program, "root"), options);
}
