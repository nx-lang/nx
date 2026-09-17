/**
 * Expands every authored component in an evaluated value tree, the way the renderer does.
 *
 * Evaluating `root` turns a component *use* into a descriptor — `{ $type: "Card", Title: … }` — and
 * leaves the body for whoever draws it, so a runtime failure inside a component body (FINDINGS
 * F22 was one) would pass a check that stopped at the descriptor. This walks the tree through the
 * same instance tree the site draws with, and walks what it meets the way `drawValue` does: each
 * authored component is initialized under the instance that encloses it, so a handler its parent
 * bound resolves through the parent, and what it rendered is walked in turn; a catalog control's
 * content is walked, and a handler without a token on it is reported, as `bind` reports it; any
 * other value is a placeholder the renderer does not look inside. What it produces is the
 * exception a body throws, or the handlers bound outside any component, which no instance can run.
 */
import meta from "../catalog/catalog-meta.json" with { type: "json" };
import { InstanceTree, isHandlerRecord } from "../src/render/instances.ts";
import { isAuthoredComponent } from "../src/render/evaluate.ts";

export function expandComponents(program, value) {
  const tree = new InstanceTree(program);
  const inert = new Set();
  tree.begin();
  walk(tree, value, "root", null, 0, inert);
  tree.finish();
  for (const where of tree.inert) {
    inert.add(where);
  }
  return { inert: [...inert] };
}

function walk(tree, value, key, parent, depth, inert) {
  if (depth > 64) {
    throw new Error("component expansion did not terminate");
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => walk(tree, item, `${key}.${index}`, parent, depth, inert));
    return;
  }
  if (typeof value !== "object" || value === null) {
    return;
  }
  const type = value.$type;
  if (typeof type !== "string") {
    return;
  }
  if (meta.components[type] === undefined) {
    if (isAuthoredComponent(tree.program, type)) {
      const node = tree.visit(key, value, parent);
      walk(tree, node.rendered, `${key}/body`, node, depth + 1, inert);
    }
    return;
  }
  for (const [name, field] of Object.entries(value)) {
    if (isHandlerRecord(field) && field.token === undefined) {
      inert.add(`${type}.${name}`);
    }
  }
  const content = value[meta.contentProperty];
  const children = content === null || content === undefined ? [] : Array.isArray(content) ? content : [content];
  children.forEach((child, index) => walk(tree, child, `${key}.${index}`, parent, depth, inert));
}
