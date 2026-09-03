import { evaluateFunction, initializeComponent, prepareNxIrProgram, type NxPreparedProgram } from "@nx-lang/ir-runtime";
import type { NxObject, NxValue } from "./values";

/** The entrypoint every fiddle program provides. */
export const ROOT_FUNCTION = "root";

export type Program = NxPreparedProgram;

/** Prepares compiled IR for evaluation. Throws on a malformed program. */
export function prepare(ir: unknown): Program {
  return prepareNxIrProgram(ir as Parameters<typeof prepareNxIrProgram>[0]);
}

/** Evaluates `root` to the value tree the renderer walks. */
export function evaluateRoot(program: Program): NxValue {
  return evaluateFunction(program, ROOT_FUNCTION) as NxValue;
}

/** Whether a type name belongs to a component the authored source declares. */
export function isAuthoredComponent(program: Program, type: string): boolean {
  const prepared = program.componentEntrypoints.get(type);
  const kind = prepared?.declaration.kind;
  return kind !== undefined && kind.tag === "component" && kind.isExternal !== true;
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
