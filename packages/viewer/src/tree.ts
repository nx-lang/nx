/**
 * What the reading asks of a source tree: each node's children, the slot it fills in its parent,
 * and each key's node.
 */
import type { SourceNode, SourceTree } from "@nx-lang/language-protocol";

/** A source tree with the lookups the reading needs, built once per tree. */
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

/**
 * The index of the node whose range has exactly the byte offsets `start` and `end`, preferring an
 * `element` node where several have them, as a record's value origin names its element. Nothing
 * when no node has them.
 */
export function nodeAtSpan(tree: SourceTree, start: number, end: number): number | undefined {
  let found: number | undefined;
  tree.nodes.forEach((node, index) => {
    if (node.range.startByte === start && node.range.endByte === end && prefers(tree, index, found)) {
      found = index;
    }
  });
  return found;
}

/** Whether the node at `candidate` wins over `found` for the same span: the first, or an element. */
function prefers(tree: SourceTree, candidate: number, found: number | undefined): boolean {
  return found === undefined || (tree.nodes[candidate]?.role === "element" && tree.nodes[found]?.role !== "element");
}
