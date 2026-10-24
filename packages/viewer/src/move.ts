/**
 * Moving and deleting the items of a list: which items move or can be deleted, decided from the
 * source tree as the language service decides what `move` and an item's `remove` take, and where a
 * key goes once an item has moved or gone. Items are keyed by their position in their list
 * (`steps[2]`), so a move changes the keys of every item between its old and new places.
 */
import type { TreeIndex } from "./tree.js";
import { attributeElement, elementProperties } from "./tree.js";

/** The list an item belongs to. */
export interface ItemList {
  /** The key of what holds the items: a sequence, an element, or an element's content. */
  readonly key: string;
  /** Its node. */
  readonly at: number;
  /** The items' nodes, in order, whatever they are: elements, `if` and `for` blocks and the rest. */
  readonly items: readonly number[];
  /** The position of the item asked about. */
  readonly position: number;
  /** Whether the list is a sequence, which reads as its one item once only one is left. */
  readonly sequence: boolean;
  /**
   * The key of what is selected once the list's last item is deleted: the element whose content it
   * is, or the row whose value it is.
   */
  readonly holder: string | undefined;
}

/** The position an item's key gives it in the list keyed `list`, as `steps[2]` gives `2`. */
function itemPosition(list: string, key: string): number | undefined {
  if (!key.startsWith(`${list}[`) || !key.endsWith("]")) {
    return undefined;
  }
  const digits = key.slice(list.length + 1, -1);
  return /^\d+$/.test(digits) ? Number(digits) : undefined;
}

/**
 * The list the element at `at` is an item of: a sequence or element content, not a condition's
 * branch or a loop's body, that holds no text and is written an item to a line or on one line, as
 * the language service takes it. Nothing for any other node.
 */
export function itemListOf(index: TreeIndex, at: number): ItemList | undefined {
  const { tree } = index;
  const node = tree.nodes[at];
  if (node?.role !== "element" || node.parent === undefined) {
    return undefined;
  }
  const ownerAt = node.parent;
  const owner = tree.nodes[ownerAt]!;
  const isList =
    owner.role === "sequence" || owner.role === "element" || (owner.role === "attribute" && owner.flags?.includes("content") === true);
  const position = itemPosition(owner.key, node.key);
  if (!isList || position === undefined) {
    return undefined;
  }
  const children = index.children[ownerAt] ?? [];
  if (children.some((child) => tree.nodes[child]!.role === "text")) {
    return undefined;
  }
  const items = children
    .flatMap((child) => {
      const found = itemPosition(owner.key, tree.nodes[child]!.key);
      return found === undefined ? [] : [{ found, child }];
    })
    .sort((a, b) => a.found - b.found)
    .map(({ child }) => child);
  const place = items.indexOf(at);
  if (place < 0 || !isWrittenAsAList(index, items)) {
    return undefined;
  }
  let holder: string | undefined;
  if (owner.role === "attribute") {
    holder = owner.parent === undefined ? undefined : tree.nodes[owner.parent]!.key;
  } else if (owner.role === "element") {
    holder = owner.key;
  } else {
    // A sequence is a value: of a row, which is selected, or of a declaration.
    holder = owner.parent === undefined ? undefined : tree.nodes[owner.parent]!.key;
  }
  return { key: owner.key, at: ownerAt, items, position: place, sequence: owner.role === "sequence", holder };
}

/**
 * Whether the items are written an item to a line, each on lines of its own with nothing else on
 * them but a line comment after it, or all on one line, apart by blank space only.
 */
function isWrittenAsAList(index: TreeIndex, items: readonly number[]): boolean {
  const { bytes, tree } = index;
  const blank = (byte: number | undefined): boolean => byte === 32 || byte === 9;
  const standsAlone = (at: number): boolean => {
    const { startByte, endByte } = tree.nodes[at]!.range;
    for (let before = startByte - 1; before >= 0 && bytes[before] !== 10; before--) {
      if (!blank(bytes[before])) {
        return false;
      }
    }
    let after = endByte;
    while (blank(bytes[after])) {
      after++;
    }
    if (bytes[after] === 47 && bytes[after + 1] === 47) {
      return true;
    }
    return bytes[after] === 10 || (bytes[after] === 13 && bytes[after + 1] === 10);
  };
  if (items.every(standsAlone)) {
    return true;
  }
  const first = tree.nodes[items[0]!]!.range.startByte;
  const last = tree.nodes[items[items.length - 1]!]!.range.endByte;
  if (bytes.subarray(first, last).includes(10)) {
    return false;
  }
  for (let next = 1; next < items.length; next++) {
    const from = tree.nodes[items[next - 1]!]!.range.endByte;
    const to = tree.nodes[items[next]!]!.range.startByte;
    for (let at = from; at < to; at++) {
      if (!blank(bytes[at])) {
        return false;
      }
    }
  }
  return true;
}

/** Whether the element at `at` moves: an item of a list of two or more. */
export function canMove(index: TreeIndex, at: number): boolean {
  return (itemListOf(index, at)?.items.length ?? 0) >= 2;
}

/**
 * Whether the element at `at` can be deleted: an item of a list, unless it is the only item of a
 * list whose type needs one or more.
 */
export function canDelete(index: TreeIndex, at: number): boolean {
  const list = itemListOf(index, at);
  return list !== undefined && (list.items.length > 1 || !needsOne(index, list));
}

/** Whether the list's type, as its place spells it, needs one item or more, as `FlowStep+` does. */
export function needsOne(index: TreeIndex, list: ItemList): boolean {
  const { tree } = index;
  const owner = tree.nodes[list.at]!;
  let spelled: string | undefined;
  if (owner.role === "attribute") {
    const element = owner.parent === undefined ? undefined : tree.nodes[owner.parent];
    spelled = element === undefined ? undefined : elementProperties(tree, element)?.find((property) => property.name === owner.name)?.type;
  } else if (owner.role === "sequence" && owner.parent !== undefined) {
    const place = tree.nodes[owner.parent]!;
    if (place.role === "attribute") {
      const element = attributeElement(tree, owner.parent);
      spelled =
        element === undefined ? undefined : elementProperties(tree, tree.nodes[element]!)?.find((property) => property.name === place.name)?.type;
    } else if (place.role === "declaration") {
      const written = (index.children[owner.parent] ?? []).find(
        (child) => index.slots[child] === "type" && tree.nodes[child]!.role === "typeReference"
      );
      spelled = written === undefined ? undefined : tree.nodes[written]!.value;
    }
  }
  // Through the aliases the tree declares, to the first `+` or `*`.
  for (let depth = 0; spelled !== undefined && depth < 8; depth++) {
    const trimmed = spelled.trim();
    if (trimmed.endsWith("+")) {
      return true;
    }
    if (trimmed.endsWith("*")) {
      return false;
    }
    const name = trimmed.replace(/\?+$/, "").trim();
    spelled = tree.declarations.find((entry) => entry.kind === "alias" && entry.name === name)?.type;
  }
  return false;
}

/**
 * Where each item of a list goes as one move or deletion follows another: the position of every
 * item as the list was, by its position now, or nothing for one deleted.
 */
export class ItemMap {
  #positions: (number | undefined)[];
  #count: number;

  constructor(count: number) {
    this.#positions = Array.from({ length: count }, (_, position) => position);
    this.#count = count;
  }

  /** How many items the list holds now. */
  get count(): number {
    return this.#count;
  }

  /** Where the item at `position`, as the list was, is now, or nothing when it was deleted. */
  positionOf(position: number): number | undefined {
    return this.#positions[position];
  }

  /** The item now at `from` moves to `to`. */
  move(from: number, to: number): void {
    this.#positions = this.#positions.map((now) => (now === undefined ? undefined : movedPosition(now, from, to)));
  }

  /** The item now at `position` is deleted. */
  delete(position: number): void {
    this.#positions = this.#positions.map((now) =>
      now === undefined || now === position ? undefined : now > position ? now - 1 : now
    );
    this.#count -= 1;
  }
}

/** Where the item at `position` goes when the item at `from` moves to `to`. */
export function movedPosition(position: number, from: number, to: number): number {
  if (position === from) {
    return to;
  }
  if (from < to && position > from && position <= to) {
    return position - 1;
  }
  if (to < from && position >= to && position < from) {
    return position + 1;
  }
  return position;
}

/**
 * The key `key` of the list keyed `list` takes once its items have moved as `map` says: an item's
 * key, or the key of a node inside an item or of a comment keyed by one, at the item's new position;
 * any other key as it is. Nothing for a key inside a deleted item. A sequence left with one item
 * reads as that item, keyed as the sequence was.
 */
export function mappedKey(key: string, list: string, map: ItemMap, sequence: boolean): string | undefined {
  const found = /^\[(\d+)\]/.exec(key.slice(list.length));
  if (!key.startsWith(list) || found === null) {
    return key;
  }
  const rest = key.slice(list.length + found[0].length);
  if (rest !== "" && !/^[.[:#]/.test(rest)) {
    return key;
  }
  const position = map.positionOf(Number(found[1]));
  if (position === undefined) {
    return undefined;
  }
  return sequence && map.count === 1 ? `${list}${rest}` : `${list}[${position}]${rest}`;
}
