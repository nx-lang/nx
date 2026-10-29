import { FILE_NAME } from "../compile/policy.ts";
import {
  authoredSites,
  selectedTargets,
  type SourceSelection,
} from "../preview/selection.ts";
import type { GraphicsNode } from "./protocol.ts";
export type { SourceSelection as DrawingSelection } from "../preview/selection.ts";
export function authoredSite(node: GraphicsNode) {
  return authoredSites(node.provenance, FILE_NAME)[0];
}
export function selectedNodes(
  nodes: readonly GraphicsNode[],
  range: SourceSelection | null,
): string[] {
  return selectedTargets(
    nodes.map((node) => ({
      id: node.id,
      sources: authoredSites(node.provenance, FILE_NAME),
    })),
    range,
    FILE_NAME,
  );
}
