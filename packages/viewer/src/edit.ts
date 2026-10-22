/**
 * What the reading can edit, and with which editor: decided from the source tree alone, as the
 * language service decides what a source edit may change. A value is editable when it is a literal
 * or a bare union case that is a whole value (an attribute's value, an item of a braced sequence,
 * or a parameterless `let`'s value), or a run of plain text written on one line, and an editor fits
 * its type. The service checks again, so this decides only what the reading offers.
 */
import type {
  SourceCase,
  SourceEdit,
  SourceEditorHints,
  SourceEditValue,
  SourceProperty,
  SourceValueFormat
} from "@nx-lang/language-protocol";

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
  /** How its property's visualizers say it edits: a slider, a range, a unit, several lines. */
  readonly hints?: SourceEditorHints;
  /**
   * The element and the property the value is set on, for a whole value of an attribute or of a
   * property the element does not write: what the host's named values are asked for.
   */
  readonly property?: { readonly element: string; readonly name: string };
  /**
   * Set for a property the element does not write, which an edit adds: its value is the
   * property's default, or nothing, and its node is the element's.
   */
  readonly adding?: boolean;
  /** The reference path the value is written as, as `theme.accent`, when it is a named value. */
  readonly reference?: string;
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
  if (target.kind === "run") {
    return { kind: "setText", text: value };
  }
  return valueEdit(target, valueOf(target, value));
}

/** The edit that sets `target` to the named value at `path`, written as a reference to it. */
export function referenceEditOf(target: EditTarget, path: string): SourceEdit {
  return valueEdit(target, { kind: "reference", path });
}

function valueEdit(target: EditTarget, value: SourceEditValue): SourceEdit {
  return target.adding === true && target.property !== undefined
    ? { kind: "addProperty", name: target.property.name, value }
    : { kind: "setValue", value };
}

function valueOf(target: EditTarget, value: string): SourceEditValue {
  switch (target.kind) {
    case "number":
      return { kind: "number", text: value };
    case "boolean":
      return { kind: "boolean", value: value === "true" };
    case "case":
      return { kind: "case", name: value };
    default:
      return { kind: "string", value };
  }
}

/** Whether `text` is a reference to a named value as an edit writes one: names joined by dots. */
export function isReferencePath(text: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(text);
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
  return removalOf(index, at) !== undefined;
}

/**
 * How the attribute at `at` is left out: `reset` to the default its property has, `remove` for an
 * optional property, or nothing for one that may not be left out.
 */
export function removalOf(index: TreeIndex, at: number): "reset" | "remove" | undefined {
  const { tree } = index;
  const attribute = tree.nodes[at];
  if (attribute?.role !== "attribute" || attribute.flags?.includes("content") === true || attribute.parent === undefined) {
    return undefined;
  }
  const element = tree.nodes[attribute.parent]!;
  const declaration = element.declaration === undefined ? undefined : tree.declarations[element.declaration];
  if (element.role !== "element" || declaration === undefined || declaration.kind === "union") {
    return undefined;
  }
  const property = declaration.properties?.find((candidate) => candidate.name === attribute.name);
  if (property?.default !== undefined) {
    return "reset";
  }
  return property?.flags?.includes("optional") === true ? "remove" : undefined;
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
  if (!isWholeValue(index, at)) {
    return undefined;
  }
  const spelled = declaredType(index, at);
  const declared = resolveType(index, spelled);
  const hints = hintsOf(index, at, spelled);
  const property = propertyOf(index, at);
  const extra = { ...(hints === undefined ? {} : { hints }), ...(property === undefined ? {} : { property }) };
  if (node.role === "reference" || node.role === "member") {
    // A named value, as `{accent}` or `theme.accent`, edits with its type's editor.
    const path = referencePath(index, at);
    const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
    // A literal written over a braced reference loses the braces, so a number may take a sign there.
    const signed = owner?.role === "attribute" || owner?.role === "declaration";
    const kind = path === undefined ? undefined : editorOfType(index, spelled, formats.get(at) ?? declared.format, signed);
    if (path === undefined || kind === undefined) {
      return undefined;
    }
    return { at, key, ...kind, value: path, reference: path, ...extra };
  }
  if (node.role !== "literal" && node.role !== "case") {
    return undefined;
  }
  if (node.role === "case") {
    const union = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
    const cases = union?.kind === "union" ? (union.cases ?? []).filter((candidate) => (candidate.properties ?? []).length === 0) : [];
    return cases.length > 0 && node.name !== undefined ? { at, key, kind: "case", value: node.name, cases, ...extra } : undefined;
  }
  const written = node.value ?? "";
  if (written.startsWith('"') && written.endsWith('"') && written.length >= 2) {
    const format = formats.get(at) ?? declared.format;
    const value = decodeString(written);
    return value === undefined ? undefined : { at, key, kind: format === "color" ? "color" : "text", value, ...extra };
  }
  if (written === "true" || written === "false") {
    return { at, key, kind: "boolean", value: written, ...extra };
  }
  if (isNumericLiteral(written) && (declared.base === undefined || NUMBER_TYPES.has(declared.base))) {
    const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
    const signed = (owner?.role === "attribute" || owner?.role === "declaration") && node.flags?.includes("braced") !== true;
    return { at, key, kind: "number", value: written, signed, ...extra };
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
 * `?`, `+` or `*`; the format and the editor hints the first alias with a visualizer gives its
 * values; and whether it is a list anywhere along the way.
 */
function resolveType(
  index: TreeIndex,
  spelled: string | undefined
): { base: string | undefined; format: SourceValueFormat | undefined; editor: SourceEditorHints | undefined; list: boolean } {
  let name = spelled?.replace(/[?+*]+$/, "").trim();
  let list = spelled !== undefined && /[+*]/.test(spelled.trim().slice(-2));
  let format: SourceValueFormat | undefined;
  let editor: SourceEditorHints | undefined;
  for (let depth = 0; name !== undefined && name !== "" && depth < 8; depth++) {
    const current: string = name;
    const alias = index.tree.declarations.find((entry) => entry.kind === "alias" && entry.name === current);
    if (alias === undefined || alias.type === undefined) {
      break;
    }
    format ??= alias.visualizer?.format;
    editor ??= alias.visualizer?.editor;
    list ||= /[+*]/.test(alias.type.trim().slice(-2));
    name = alias.type.replace(/[?+*]+$/, "").trim();
  }
  return { base: name === "" ? undefined : name, format, editor, list };
}

/** The editor a value of the type `spelled` opens, and what it needs: nothing for a type no editor takes. */
export interface TypeEditor {
  readonly kind: Exclude<EditorKind, "run">;
  readonly cases?: readonly SourceCase[];
  readonly signed?: boolean;
}

/** The types a value of which reads as a boolean. */
const BOOLEAN_TYPES = new Set(["boolean", "bool"]);

/**
 * The editor a value of the type `spelled` opens, formatted as `format` says: a number field, a
 * toggle, a text field or a color's, or a union's cases where it has cases with no properties.
 * Nothing for a list, a record, an element, a function or a type the tree does not know.
 */
export function editorOfType(
  index: TreeIndex,
  spelled: string | undefined,
  format: SourceValueFormat | undefined,
  signed = true
): TypeEditor | undefined {
  const declared = resolveType(index, spelled);
  const base = declared.base;
  if (base === undefined || declared.list) {
    return undefined;
  }
  if (NUMBER_TYPES.has(base)) {
    return { kind: "number", signed };
  }
  if (BOOLEAN_TYPES.has(base)) {
    return { kind: "boolean" };
  }
  if (base === "string") {
    return { kind: (format ?? declared.format) === "color" ? "color" : "text" };
  }
  const union = index.tree.declarations.find((entry) => entry.kind === "union" && entry.name === base);
  const cases = (union?.cases ?? []).filter((candidate) => (candidate.properties ?? []).length === 0);
  return cases.length > 0 ? { kind: "case", cases } : undefined;
}

/**
 * The editor hints of the whole value at `at`, whose place declares `spelled`: its property's, as
 * its element's visualizer resolves them, or else those of the alias its type names.
 */
function hintsOf(index: TreeIndex, at: number, spelled: string | undefined): SourceEditorHints | undefined {
  const { tree } = index;
  let place = tree.nodes[at]!.parent;
  while (place !== undefined && tree.nodes[place]!.role === "sequence") {
    place = tree.nodes[place]!.parent;
  }
  const attribute = place === undefined ? undefined : tree.nodes[place];
  if (attribute?.role === "attribute" && attribute.parent !== undefined) {
    const element = tree.nodes[attribute.parent]!;
    const declaration = element.declaration === undefined ? undefined : tree.declarations[element.declaration];
    const own = declaration?.visualizer?.properties?.find((candidate) => candidate.name === attribute.name)?.editor;
    if (own !== undefined) {
      return own;
    }
  }
  return resolveType(index, spelled).editor;
}

/** The element and the property whose value the node at `at` is: an attribute's whole value, not an item. */
function propertyOf(index: TreeIndex, at: number): EditTarget["property"] {
  const { tree } = index;
  const attribute = tree.nodes[at]!.parent === undefined ? undefined : tree.nodes[tree.nodes[at]!.parent!];
  if (attribute?.role !== "attribute" || attribute.parent === undefined || attribute.name === undefined) {
    return undefined;
  }
  if (attribute.flags?.includes("content") === true) {
    return undefined;
  }
  return { element: tree.nodes[attribute.parent]!.key, name: attribute.name };
}

/** The reference path the node at `at` is written as, its braces left out, or nothing when it is not one. */
function referencePath(index: TreeIndex, at: number): string | undefined {
  const node = index.tree.nodes[at]!;
  const written = new TextDecoder()
    .decode(index.bytes.subarray(node.range.startByte, node.range.endByte))
    .trim()
    .replace(/^\{\s*([^{}]*?)\s*\}$/, "$1");
  return isReferencePath(written) ? written : undefined;
}

/** A property an element does not write, as the reading offers it to add. */
export interface UnwrittenProperty {
  readonly property: SourceProperty;
  /** How its value edits, or nothing when no editor takes its type, as a record's or a list's. */
  readonly editor: TypeEditor | undefined;
  readonly hints: SourceEditorHints | undefined;
  readonly format: SourceValueFormat | undefined;
}

/**
 * The properties the element at `at` declares and does not write, its content left out, in the
 * order its visualizer lists them, else in declaration order. Nothing for a node that is not an
 * element, or an element of a union's case.
 */
export function unwrittenProperties(index: TreeIndex, at: number): UnwrittenProperty[] {
  const { tree } = index;
  const node = tree.nodes[at];
  const declaration = node?.declaration === undefined ? undefined : tree.declarations[node.declaration];
  if (node?.role !== "element" || declaration === undefined || declaration.kind === "union") {
    return [];
  }
  const written = new Set(
    (index.children[at] ?? []).flatMap((child) => (tree.nodes[child]!.role === "attribute" ? [tree.nodes[child]!.name ?? ""] : []))
  );
  const properties = (declaration.properties ?? []).filter(
    (property) => property.flags?.includes("content") !== true && !written.has(property.name)
  );
  const order = (declaration.visualizer?.properties ?? []).map((candidate) => candidate.name);
  const rank = (name: string): number => {
    const found = order.indexOf(name);
    return found < 0 ? order.length : found;
  };
  return properties
    .map((property, position) => ({ property, position }))
    .sort((a, b) => rank(a.property.name) - rank(b.property.name) || a.position - b.position)
    .map(({ property }) => {
      const visualizer = declaration.visualizer?.properties?.find((candidate) => candidate.name === property.name);
      const format = visualizer?.format ?? resolveType(index, property.type).format;
      return {
        property,
        editor: editorOfType(index, property.type, format),
        hints: visualizer?.editor ?? resolveType(index, property.type).editor,
        format
      };
    });
}
