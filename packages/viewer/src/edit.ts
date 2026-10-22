/**
 * What the reading can edit, and with which editor: decided from the source tree alone, as the
 * language service decides what a source edit may change. A value is editable when it is a literal
 * or a bare union case that is a whole value (an attribute's value, an item of a braced sequence,
 * or a parameterless `let`'s value), or a run of plain text written on one line, and an editor fits
 * its type. The service checks again, so this decides only what the reading offers.
 */
import type { SourceCase, SourceEdit, SourceValueFormat } from "@nx-lang/language-protocol";

import { decodeString, formattedNodes } from "./source.js";
import type { TreeIndex } from "./tree.js";

/**
 * The editor a value opens: a text field (strings, UI text and ids), a number field, a toggle, a
 * popover of a union's cases, a popover with a color picker, or a one-line field for a run of text.
 */
export type EditorKind = "text" | "number" | "boolean" | "case" | "color" | "run";

/** One editable value of the reading. */
export interface EditTarget {
  /** The index of its node in the tree. */
  readonly at: number;
  readonly key: string;
  readonly kind: EditorKind;
  /**
   * What the editor starts with: a string's content without its quotes and escapes, a number or a
   * run as written, a case's name, or `true` or `false`.
   */
  readonly value: string;
  /** The cases a case's popover lists: those of its union that hold no properties. */
  readonly cases?: readonly SourceCase[];
  /**
   * Whether a number may be negative here: only an unbraced attribute or `let` value takes a sign,
   * since in a sequence or inside braces `-` reads as an operator.
   */
  readonly signed?: boolean;
}

/** The types a number field edits, and their aliases' ends. */
const NUMBER_TYPES = new Set(["int", "float", "float32", "float64"]);

/** Whether `text` is an NX numeric literal: an integer, a real or a hex literal, with a leading minus. */
export function isNumericLiteral(text: string): boolean {
  return /^-?(?:0[xX][0-9a-fA-F]+|\d+(?:\.\d+)?|\d+\.\d+[eE][+-]?\d+)$/.test(text);
}

/**
 * A run of text content, as the grammar reads one: it starts with neither a space nor `<` or `{`,
 * holds neither `<` nor `{`, and does not start with `if`, `for` or `else`, or the start of one of
 * them, as a word, which a block takes.
 */
const TEXT_RUN =
  /^(?:[^\s<{ife][^<{]*|i[^f\s<{][^<{]*|if[^\s<{][^<{]*|f[^o\s<{][^<{]*|fo[^r\s<{][^<{]*|for[^\s<{][^<{]*|e[^l\s<{][^<{]*|el[^s\s<{][^<{]*|els[^e\s<{][^<{]*|else[^\s<{][^<{]*)$/;

/**
 * Why a run of text cannot be written as typed, or nothing when it can: element content has no
 * escapes, so what would start a tag, a braced value or a block cannot be in a run, and a run
 * cannot start with a space or hold a line break. The language service checks again.
 */
export function runProblem(text: string): string | undefined {
  if (text.trim() === "") {
    return "Text cannot be empty";
  }
  if (/[\r\n]/.test(text)) {
    return "Text takes one line";
  }
  if (/^\s/.test(text)) {
    return "Text cannot start with a space";
  }
  if (/[<{]/.test(text)) {
    return "Text cannot hold < or {";
  }
  if (!TEXT_RUN.test(text)) {
    return "Text cannot start with if, for or else, or the start of one, as a word";
  }
  return undefined;
}

/** Whether `text` is a color a hex field takes: `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`. */
export function isHexColor(text: string): boolean {
  return /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(text);
}

/** The edit that sets `target` to `value`, as its editor holds it. */
export function editOf(target: EditTarget, value: string): SourceEdit {
  switch (target.kind) {
    case "text":
    case "color":
      return { kind: "setValue", value: { kind: "string", value } };
    case "number":
      return { kind: "setValue", value: { kind: "number", text: value } };
    case "boolean":
      return { kind: "setValue", value: { kind: "boolean", value: value === "true" } };
    case "case":
      return { kind: "setValue", value: { kind: "case", name: value } };
    case "run":
      return { kind: "setText", text: value };
  }
}

/** Every editable value of the tree, by key. */
export function editTargets(index: TreeIndex): Map<string, EditTarget> {
  const { tree } = index;
  const formats = formattedNodes(index);
  const targets = new Map<string, EditTarget>();
  tree.nodes.forEach((node, at) => {
    const target = targetOf(index, at, formats);
    if (target !== undefined) {
      targets.set(node.key, target);
    }
  });
  return targets;
}

/**
 * Whether the attribute at `at` may be left out: its property has a default, which it goes back to,
 * or is optional.
 */
export function isRemovable(index: TreeIndex, at: number): boolean {
  const { tree } = index;
  const attribute = tree.nodes[at];
  if (attribute?.role !== "attribute" || attribute.flags?.includes("content") === true || attribute.parent === undefined) {
    return false;
  }
  const element = tree.nodes[attribute.parent]!;
  const declaration = element.declaration === undefined ? undefined : tree.declarations[element.declaration];
  if (element.role !== "element" || declaration === undefined || declaration.kind === "union") {
    return false;
  }
  const property = declaration.properties?.find((candidate) => candidate.name === attribute.name);
  return property !== undefined && (property.default !== undefined || property.flags?.includes("optional") === true);
}

function targetOf(
  index: TreeIndex,
  at: number,
  formats: ReadonlyMap<number, SourceValueFormat>
): EditTarget | undefined {
  const { tree } = index;
  const node = tree.nodes[at]!;
  const key = node.key;
  if (node.role === "text") {
    // The space after a run separates it from what follows, a braced value or an element, and stays.
    const value = (node.value ?? "").trimEnd();
    return isPlainRun(index, at) && value !== "" ? { at, key, kind: "run", value } : undefined;
  }
  if ((node.role !== "literal" && node.role !== "case") || !isWholeValue(index, at)) {
    return undefined;
  }
  const declared = resolveType(index, declaredType(index, at));
  if (node.role === "case") {
    const union = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
    const cases = union?.kind === "union" ? (union.cases ?? []).filter((candidate) => (candidate.properties ?? []).length === 0) : [];
    return cases.length > 0 && node.name !== undefined ? { at, key, kind: "case", value: node.name, cases } : undefined;
  }
  const written = node.value ?? "";
  if (written.startsWith('"') && written.endsWith('"') && written.length >= 2) {
    const format = formats.get(at) ?? declared.format;
    const value = decodeString(written);
    return value === undefined ? undefined : { at, key, kind: format === "color" ? "color" : "text", value };
  }
  if (written === "true" || written === "false") {
    return { at, key, kind: "boolean", value: written };
  }
  if (isNumericLiteral(written) && (declared.base === undefined || NUMBER_TYPES.has(declared.base))) {
    const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
    const signed = (owner?.role === "attribute" || owner?.role === "declaration") && node.flags?.includes("braced") !== true;
    return { at, key, kind: "number", value: written, signed };
  }
  // A literal of any other kind, or a number where something else is declared, reads as it is.
  return undefined;
}

/**
 * Whether the node at `at` is a whole value: an attribute's value, the value of a `let` that takes
 * no parameters, or an item of a braced sequence that is itself a whole value.
 */
export function isWholeValue(index: TreeIndex, at: number): boolean {
  const { tree } = index;
  const node = tree.nodes[at]!;
  const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
  if (owner === undefined) {
    return false;
  }
  switch (owner.role) {
    case "attribute":
      return node.key === `${owner.key}.value`;
    case "declaration":
      return (
        node.key === `${owner.key}.value` &&
        !(index.children[node.parent!] ?? []).some((child) => tree.nodes[child]!.role === "parameter")
      );
    case "sequence":
      return node.key.startsWith(`${owner.key}[`) && isWholeValue(index, node.parent!);
    default:
      return false;
  }
}

/** Whether the text node at `at` is a run of plain text on one line, outside typed and raw bodies. */
function isPlainRun(index: TreeIndex, at: number): boolean {
  const { tree } = index;
  const node = tree.nodes[at]!;
  if (node.textType !== undefined || node.flags?.includes("raw") === true || node.range.start.line !== node.range.end.line) {
    return false;
  }
  // The run is in an element's body; a typed body, as `<Note:markdown>`, is the element's.
  for (let parent = node.parent; parent !== undefined; parent = tree.nodes[parent]!.parent) {
    const holder = tree.nodes[parent]!;
    if (holder.textType !== undefined || holder.flags?.includes("raw") === true) {
      return false;
    }
    if (holder.role === "element") {
      break;
    }
  }
  return true;
}

/**
 * The type the place of the whole value at `at` declares, as spelled: its property's, its `let`'s,
 * or for an item, its list's. Nothing where none is declared.
 */
function declaredType(index: TreeIndex, at: number): string | undefined {
  const { tree } = index;
  const node = tree.nodes[at]!;
  const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
  switch (owner?.role) {
    case "attribute": {
      const element = owner.parent === undefined ? undefined : tree.nodes[owner.parent];
      const declaration = element?.declaration === undefined ? undefined : tree.declarations[element.declaration];
      return declaration?.properties?.find((property) => property.name === owner.name)?.type;
    }
    case "declaration": {
      const written = (index.children[node.parent!] ?? []).find(
        (child) => index.slots[child] === "type" && tree.nodes[child]!.role === "typeReference"
      );
      return written === undefined ? undefined : tree.nodes[written]!.value;
    }
    case "sequence":
      return declaredType(index, node.parent!);
    default:
      return undefined;
  }
}

/**
 * A spelled type followed through the aliases the tree declares: the type it ends at, without its
 * `?`, `+` or `*`, and the format the first alias with a visualizer gives its values.
 */
function resolveType(
  index: TreeIndex,
  spelled: string | undefined
): { base: string | undefined; format: SourceValueFormat | undefined } {
  let name = spelled?.replace(/[?+*]+$/, "").trim();
  let format: SourceValueFormat | undefined;
  for (let depth = 0; name !== undefined && name !== "" && depth < 8; depth++) {
    const current: string = name;
    const alias = index.tree.declarations.find((entry) => entry.kind === "alias" && entry.name === current);
    if (alias === undefined || alias.type === undefined) {
      break;
    }
    format ??= alias.visualizer?.format;
    name = alias.type.replace(/[?+*]+$/, "").trim();
  }
  return { base: name === "" ? undefined : name, format };
}
