import type { TreeIndex } from "./tree.js";
import { itemListOf } from "./move.js";

/** Deliberate property disclosure choices, independent of child folds and source history. */
export interface NxDetailsState {
  readonly opened: readonly string[];
  readonly pinned: readonly string[];
  readonly suppressed?: string | undefined;
}

export const emptyDetails = (): NxDetailsState => ({ opened: [], pinned: [] });

/** An element in the composition, rather than a record inside another element's property. */
export function isComposition(index: TreeIndex, at: number): boolean {
  if (index.tree.nodes[at]?.role !== "element") {
    return false;
  }
  for (let parent = index.tree.nodes[at]!.parent; parent !== undefined; parent = index.tree.nodes[parent]!.parent) {
    const node = index.tree.nodes[parent]!;
    if (node.role === "attribute" && !node.flags?.includes("content")) {
      return false;
    }
  }
  return true;
}

/** The composition element whose details contain a selection, including nested property values. */
export function compositionOwner(index: TreeIndex, key: string | undefined): string | undefined {
  for (let at = key === undefined ? undefined : index.byKey.get(key); at !== undefined; at = index.tree.nodes[at]!.parent) {
    if (isComposition(index, at)) {
      return index.tree.nodes[at]!.key;
    }
  }
  return undefined;
}

export function detailsOpen(state: NxDetailsState, owner: string, selected: string | undefined): boolean {
  return state.opened.includes(owner) || state.pinned.includes(owner) || (owner === selected && state.suppressed !== owner);
}

export function changeDetails(
  state: NxDetailsState, key: string, action: "details" | "pin", selected: string | undefined
): NxDetailsState {
  const opened = new Set(state.opened);
  const pinned = new Set(state.pinned);
  let suppressed = state.suppressed;
  if (action === "pin") {
    if (pinned.has(key)) {
      pinned.delete(key);
    } else {
      pinned.add(key);
      if (suppressed === key) {
        suppressed = undefined;
      }
    }
  } else if (detailsOpen(state, key, selected)) {
    opened.delete(key);
    pinned.delete(key);
    if (selected === key) {
      suppressed = key;
    }
  } else {
    opened.add(key);
    if (suppressed === key) {
      suppressed = undefined;
    }
  }
  return { opened: [...opened], pinned: [...pinned], suppressed };
}

export function mapDetails(state: NxDetailsState, map: (key: string) => string | undefined): NxDetailsState {
  const keys = (values: readonly string[]) => [...new Set(values.flatMap((key) => {
    const next = map(key);
    return next === undefined ? [] : [next];
  }))].sort();
  return {
    opened: keys(state.opened),
    pinned: keys(state.pinned),
    suppressed: state.suppressed === undefined ? undefined : map(state.suppressed)
  };
}

export function sameDetails(a: NxDetailsState, b: NxDetailsState): boolean {
  return a.suppressed === b.suppressed && a.opened.length === b.opened.length && a.pinned.length === b.pinned.length &&
    a.opened.every((key) => b.opened.includes(key)) && a.pinned.every((key) => b.pinned.includes(key));
}

/** Preserve unambiguous list identities across Source edits, including undo of known moves. */
export function reconcileDetails(previous: TreeIndex, next: TreeIndex, state: NxDetailsState): NxDetailsState {
  const written = (index: TreeIndex, at: number): string => {
    const range = index.tree.nodes[at]!.range;
    return new TextDecoder().decode(index.bytes.subarray(range.startByte, range.endByte));
  };
  return mapDetails(state, (key) => {
    let mapped = key;
    // Walk outward: a pin in a nested container must also follow a moved ancestor.
    for (let at = previous.byKey.get(key); at !== undefined; at = previous.tree.nodes[at]!.parent) {
      const list = itemListOf(previous, at);
      if (list === undefined) {
        continue;
      }
      const nextAt = next.byKey.get(list.key);
      if (nextAt === undefined) {
        return undefined;
      }
      const candidates = (next.children[nextAt] ?? []).filter((item) =>
        /^\[\d+\]$/.test(next.tree.nodes[item]!.key.slice(list.key.length))
      );
      const oldTexts = list.items.map((item) => written(previous, item));
      const newTexts = candidates.map((item) => written(next, item));
      if (oldTexts.length === newTexts.length && oldTexts.every((value, position) => value === newTexts[position])) {
        continue;
      }
      // A single changed item with the same shape is an ordinary property edit.
      const differences = oldTexts.flatMap((value, position) => value === newTexts[position] ? [] : [position]);
      if (oldTexts.length === newTexts.length && differences.length === 1) {
        const position = differences[0]!;
        const oldNode = previous.tree.nodes[list.items[position]!]!;
        const newNode = next.tree.nodes[candidates[position]!]!;
        if (oldNode.role === newNode.role && oldNode.name === newNode.name) {
          continue;
        }
      }
      const text = written(previous, at);
      const matches = newTexts.flatMap((value, position) => value === text ? [position] : []);
      if (matches.length !== 1 || oldTexts.filter((value) => value === text).length !== 1) {
        return undefined;
      }
      const oldKey = previous.tree.nodes[at]!.key;
      const newKey = next.tree.nodes[candidates[matches[0]!]!]!.key;
      mapped = newKey + mapped.slice(oldKey.length);
    }
    return next.byKey.has(mapped) ? mapped : undefined;
  });
}
