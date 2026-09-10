/**
 * Expands every authored component in an evaluated value tree, the way the renderer does.
 *
 * Evaluating `root` turns a component *use* into a descriptor — `{ $type: "Card", Title: … }` — and
 * leaves the body for whoever draws it, so a runtime failure inside a component body (FINDINGS F22
 * was one) would pass a check that stopped at the descriptor. This walks the tree and initializes
 * each authored component with the props its use supplies, then walks what it rendered. It returns
 * nothing: what it produces is the exception a body throws, or silence.
 */
import { initializeComponent } from "@nx-lang/ir-runtime";

export function expandComponents(program, value, depth = 0) {
  if (depth > 64) {
    throw new Error("component expansion did not terminate");
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      expandComponents(program, item, depth);
    }
    return;
  }
  if (typeof value !== "object" || value === null) {
    return;
  }
  const type = value.$type;
  const kind = typeof type === "string" ? program.componentEntrypoints.get(type)?.declaration.kind : undefined;
  if (kind !== undefined && kind.tag === "component" && kind.isExternal !== true) {
    const props = {};
    for (const [name, prop] of Object.entries(value)) {
      if (name !== "$type" && prop !== undefined) {
        props[name] = prop;
      }
    }
    expandComponents(program, initializeComponent(program, type, props).rendered, depth + 1);
    return;
  }
  for (const [name, field] of Object.entries(value)) {
    if (name !== "$type") {
      expandComponents(program, field, depth);
    }
  }
}
