/**
 * What the renderers ask of a source tree: each node's children, the slot it fills in its parent,
 * its declaration entry, and the exact text of its range.
 */
import type { SourceDeclaration, SourceNode, SourceTree } from "@nx-lang/language-protocol";

/** A source tree with the lookups the renderers need, built once per tree. */
export interface TreeIndex {
  readonly tree: SourceTree;
  readonly text: string;
  /** The indices of each node's children, in source order. */
  readonly children: readonly (readonly number[])[];
  /**
   * The slot each node fills in its parent, read from its key: `test`, `then`, `else`, `left`,
   * `right`, `operand`, `value`, `default`, `type`, `body`, `args`, `pattern`, `in` and so on, or
   * an empty string for a positional item. A comment takes the slot of the node it precedes.
   */
  readonly slots: readonly string[];
  /** Each key's node. */
  readonly byKey: ReadonlyMap<string, number>;
  /** The document's text as UTF-8, which ranges' byte offsets count in. */
  readonly bytes: Uint8Array;
}

/** A suffix that keys a node naming nothing, a comment or an unparsed region, by its sibling. */
const SIBLING_KEYED = /:(?:comment|docComment|unparsed)\[\d+\]$/;

/** Indexes `tree`, whose document's text is `text`. */
export function indexTree(tree: SourceTree, text: string): TreeIndex {
  const nodes = tree.nodes;
  const children: number[][] = nodes.map(() => []);
  const byKey = new Map<string, number>();
  nodes.forEach((node, index) => {
    if (node.parent !== undefined) {
      children[node.parent]?.push(index);
    }
    byKey.set(node.key, index);
  });
  const slots = nodes.map((node) => slotOf(node, node.parent === undefined ? undefined : nodes[node.parent]));
  return { tree, text, children, slots, byKey, bytes: new TextEncoder().encode(text) };
}

function slotOf(node: SourceNode, parent: SourceNode | undefined): string {
  if (parent === undefined) {
    return "";
  }
  const key = node.key.replace(SIBLING_KEYED, "");
  if (!key.startsWith(parent.key)) {
    return "";
  }
  const rest = key.slice(parent.key.length);
  if (!rest.startsWith(".")) {
    return "";
  }
  return rest.slice(1).split(/[.[]/)[0] ?? "";
}

/** The node at `index`. */
export function nodeOf(index: TreeIndex, at: number): SourceNode {
  const node = index.tree.nodes[at];
  if (node === undefined) {
    throw new RangeError(`No node ${at} in the source tree.`);
  }
  return node;
}

/** The declaration entry a node refers to, if any. */
export function declarationOf(index: TreeIndex, node: SourceNode): SourceDeclaration | undefined {
  return node.declaration === undefined ? undefined : index.tree.declarations[node.declaration];
}

/** The children of `at` that fill `slot`. */
export function childrenIn(index: TreeIndex, at: number, slot: string): number[] {
  return (index.children[at] ?? []).filter((child) => index.slots[child] === slot);
}

/** The exact text of the node at `at`, sliced by its range's UTF-8 byte offsets. */
export function sourceTextOf(index: TreeIndex, at: number): string {
  const range = nodeOf(index, at).range;
  return new TextDecoder().decode(index.bytes.subarray(range.startByte, range.endByte));
}

/**
 * The index of the node whose range has exactly the byte offsets `start` and `end`, preferring an
 * `element` node where several have them, as a record's value origin names its element. Nothing
 * when no node has them.
 */
export function nodeAtSpan(tree: SourceTree, start: number, end: number): number | undefined {
  let found: number | undefined;
  tree.nodes.forEach((node, index) => {
    if (node.range.startByte !== start || node.range.endByte !== end) {
      return;
    }
    if (found === undefined || (node.role === "element" && tree.nodes[found]?.role !== "element")) {
      found = index;
    }
  });
  return found;
}
