import {
  evaluateFunction,
  initializeComponent,
  linkNxIrProgram,
  prepareNxIrModule,
  type NxPreparedModule,
  type NxPreparedProgram,
} from "@nx-lang/ir-runtime";
import type { NxObject, NxValue } from "./values";

/** The entrypoint every playground program provides. */
export const ROOT_FUNCTION = "root";

export type Program = NxPreparedProgram;

/**
 * Prepares the catalog's artifact, the one the site bundles at build time.
 *
 * Preparation validates the artifact and indexes its declarations; done once per page, every
 * compile links against the result rather than carrying the catalog with it.
 */
export function prepareCatalog(artifact: Uint8Array): NxPreparedModule {
  return prepareNxIrModule(artifact);
}

/**
 * Prepares a compiled snippet for evaluation: its artifact, linked against the prepared catalog.
 *
 * The snippet's module table names the catalog it was compiled against. The link is strict: the
 * snippet and the catalog artifact come from the same bundle, built from the same catalog text, so
 * a version or declaration the snippet names and the catalog lacks is an application fault rather
 * than a condition to tolerate. Throws on a malformed artifact or a failed link.
 */
export function prepare(ir: Uint8Array, catalog: NxPreparedModule): Program {
  const snippet = prepareNxIrModule(ir);
  return linkNxIrProgram(snippet, {
    resolve: (identity) => (identity === catalog.identity ? catalog : undefined),
  });
}

/** Evaluates `root` to the value tree the renderer walks. */
export function evaluateRoot(program: Program): NxValue {
  return evaluateFunction(program, ROOT_FUNCTION) as NxValue;
}

/** Whether a type name belongs to a component the authored source declares. */
export function isAuthoredComponent(program: Program, type: string): boolean {
  const kind = program.componentEntrypoints.get(type)?.kind;
  return kind !== undefined && kind.tag === "component" && !kind.isExternal;
}

/**
 * Renders an author-defined component to the controls it produces.
 *
 * A descriptor for a component the author wrote — the `Demo`, `Card` and `Tile` wrappers every
 * DrawnUI demo page is built from — evaluates to `{ $type: "Demo", ... }` rather than to what the
 * component draws, because descriptor construction is deliberately atomic. Expanding it is the
 * renderer's job, and `initializeComponent` is the call that does it: it resolves the component's
 * own state block to its initial values and renders the body once.
 */
export function renderAuthoredComponent(program: Program, type: string, node: NxObject): NxValue {
  const props: Record<string, NxValue> = {};
  for (const [name, value] of Object.entries(node)) {
    if (name !== "$type" && value !== undefined) {
      props[name] = value;
    }
  }
  return initializeComponent(program, type, props as never).rendered as NxValue;
}
