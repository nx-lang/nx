import {
  declarationAtOrigin,
  type NxCanonicalValue,
  type NxOriginEntry,
  type NxPreparedProgram,
} from "@nx-lang/ir-runtime";
import type { SourceDeclaration } from "@nx-lang/language-protocol";

/** Host implementations are bundled separately from the library's type-to-renderer metadata. */
export interface Previewer {
  readonly id: string;
  readonly renderer: string;
}

export const PREVIEWERS: readonly Previewer[] = [
  { id: "graphics", renderer: "nx.graphics" },
];

export function previewerFor(
  program: NxPreparedProgram,
  value: NxCanonicalValue,
  origins: readonly NxOriginEntry[],
  declarations: readonly SourceDeclaration[],
  registry: readonly Previewer[] = PREVIEWERS,
): string | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const origin = origins.find((entry) => entry.path === "");
  const declaration =
    origin === undefined ? undefined : declarationAtOrigin(program, origin);
  const requested = declarations.find(
    (entry) =>
      entry.module === declaration?.module.identity &&
      entry.name === declaration.name,
  )?.visualizer?.preview?.renderer;
  return registry.find((entry) => entry.renderer === requested)?.id ?? null;
}
