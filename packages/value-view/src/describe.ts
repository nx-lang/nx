/**
 * What hovering a node says when the host says nothing: the node's own facts, spelled as the NX
 * language service spells hovers, a fenced `nx` fragment with a parenthesized kind where NX has no
 * spelling for the thing on its own.
 */
import type { NxValueNode, NxValueText } from "./types.js";

/** The default hover markdown for the node at `index` of `value`. */
export function defaultDescription(value: NxValueText, index: number): string {
  const node = value.nodes[index]!;
  switch (node.role) {
    case "record":
      return fenced(node.type ?? "record");
    case "property":
      return fenced(
        `(property) ${node.name ?? ""}${node.optional === true ? "?" : ""}: ${node.type ?? "object"}`
      );
    case "case": {
      const written = value.text.slice(node.start, node.end);
      const caseName = written.slice(written.lastIndexOf(".") + 1);
      return fenced(`(case) ${node.type === undefined ? "" : `${node.type}.`}${caseName}`);
    }
    case "sequence": {
      const count = node.count ?? 0;
      return `${fenced(node.type ?? "object*")}\n\n${count} ${count === 1 ? "item" : "items"}`;
    }
    case "function":
      return fenced(`(function) ${node.name ?? value.text.slice(node.start, node.end)}`);
    case "empty":
      return `${fenced("{}")}\n\nThe empty value`;
    case "scalar":
      return fenced(node.type ?? "object");
  }
}

function fenced(nx: string): string {
  return "```nx\n" + nx + "\n```";
}
