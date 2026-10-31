/**
 * What the reading can edit, and with which editor: decided from the source tree alone, as the
 * language service decides what a source edit may change. A value is editable when it is a literal
 * or a bare union case that is a whole value (an attribute's value, an item of a braced sequence,
 * or a parameterless `let`'s value), or a run of plain text written on one line, and an editor fits
 * its type. The service checks again, so this decides only what the reading offers.
 */
import type {
  SourceBounds,
  SourceCase,
  SourceEdit,
  SourceEditorHints,
  SourceEditValue,
  SourceProperty,
  SourceValueFormat
} from "@nx-lang/language-protocol";

import { decodeString, formattedNodes } from "./source.js";
import { attributeElement, elementProperties, type TreeIndex } from "./tree.js";

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
  /** A native hexadecimal literal rather than a formatted string. */
  readonly colorLiteral?: boolean;
  /** The cases a case's popover lists: those of its union that hold no properties. */
  readonly cases?: readonly SourceCase[];
  /**
   * Whether a number may be negative here: only an unbraced attribute or `let` value takes a sign,
   * since in a sequence or inside braces `-` reads as an operator.
   */
  readonly signed?: boolean;
  /**
   * How its property's visualizers say it edits: a range, a unit, choices, several lines. A number's
   * range is narrowed to its type's constraint.
   */
  readonly hints?: EditorHints;
  /**
   * The element and the property the value is set on, for a whole value of an attribute or of a
   * property the element does not write.
   */
  readonly property?: { readonly element: string; readonly name: string };
  /**
   * Set for a property the element does not write, which an edit adds: its value is the
   * property's default, or nothing, and its node is the element's.
   */
  readonly adding?: boolean;
}

/** The types a number field edits, and their aliases' ends. */
const NUMBER_TYPES = new Set(["int", "int32", "int64", "float32", "float64"]);

/** The floating-point types, whose step a constraint's span gives when no hint does. */
const FLOAT_TYPES = new Set(["float32", "float64"]);

/**
 * Editor hints with a constraint's bounds folded into their range: `minExclusive` and
 * `maxExclusive` mark an end the value may not reach.
 */
export interface EditorHints extends SourceEditorHints {
  readonly minExclusive?: boolean;
  readonly maxExclusive?: boolean;
  /** Whether the number is a `float32`, which a value and the bounds are compared as. */
  readonly float32?: boolean;
}

/**
 * The hints a number of the type `base` edits with, given its constraint's `bounds`: the range of
 * the hints narrowed to the bounds, or the bounds alone where the hints give no range or one that
 * does not overlap them. A floating-point number with no `step` hint and two ends steps by the
 * largest power of ten not above a tenth of the span. The hints as they are without bounds.
 */
export function boundedHints(
  hints: SourceEditorHints | undefined,
  bounds: SourceBounds | undefined,
  base: string | undefined
): EditorHints | undefined {
  if (bounds === undefined || (bounds.min === undefined && bounds.max === undefined)) {
    return hints;
  }
  const { min: _min, max: _max, ...rest } = hints ?? {};
  // A `float32` bound arrives as the `float64` it widens to, `0.10000000149011612`, and reads as
  // the shortest number that rounds to it, `0.1`.
  const float32 = base === "float32";
  const end = (value: number | undefined): number | undefined =>
    value === undefined || !float32 ? value : shortestFloat32(value);
  let low = end(bounds.min);
  let high = end(bounds.max);
  let lowExclusive = bounds.minExclusive === true;
  let highExclusive = bounds.maxExclusive === true;
  const hintLow = hints?.min;
  const hintHigh = hints?.max;
  const below = (value: number): boolean => low !== undefined && (value < low || (value === low && lowExclusive));
  const above = (value: number): boolean => high !== undefined && (value > high || (value === high && highExclusive));
  const empty = hintLow !== undefined && hintHigh !== undefined && hintLow > hintHigh;
  const overlaps = !empty && !(hintHigh !== undefined && below(hintHigh)) && !(hintLow !== undefined && above(hintLow));
  if (overlaps) {
    if (hintLow !== undefined && (low === undefined || hintLow > low)) {
      low = hintLow;
      lowExclusive = false;
    }
    if (hintHigh !== undefined && (high === undefined || hintHigh < high)) {
      high = hintHigh;
      highExclusive = false;
    }
  }
  let step = hints?.step;
  if (step === undefined && base !== undefined && FLOAT_TYPES.has(base) && bounds.min !== undefined && bounds.max !== undefined && bounds.max > bounds.min) {
    step = 10 ** Math.floor(Math.log10((bounds.max - bounds.min) / 10));
  }
  return {
    ...rest,
    ...(low === undefined ? {} : { min: low }),
    ...(high === undefined ? {} : { max: high }),
    ...(step === undefined ? {} : { step }),
    ...(lowExclusive ? { minExclusive: true } : {}),
    ...(highExclusive ? { maxExclusive: true } : {}),
    ...(float32 ? { float32: true } : {})
  };
}

/** The shortest number that rounds to the same `float32` as `value`: `0.1` for `0.10000000149011612`. */
function shortestFloat32(value: number): number {
  const target = Math.fround(value);
  if (!Number.isFinite(target) || target === 0) {
    return target;
  }
  for (let precision = 1; precision <= 9; precision += 1) {
    const candidate = Number(target.toPrecision(precision));
    if (Math.fround(candidate) === target) {
      return candidate;
    }
  }
  return target;
}

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

function valueEdit(target: EditTarget, value: SourceEditValue): SourceEdit {
  return target.adding === true && target.property !== undefined
    ? { kind: "addProperty", name: target.property.name, value }
    : { kind: "setValue", value };
}

function valueOf(target: EditTarget, value: string): SourceEditValue {
  if (target.colorLiteral === true) return { kind: "color", text: value };
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
  if (element.role !== "element") {
    return undefined;
  }
  const property = elementProperties(tree, element)?.find((candidate) => candidate.name === attribute.name);
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
  if ((node.role === "call" || node.role === "element") && node.color !== undefined) return { at, key, kind: "color", value: node.color, colorLiteral: true, ...extra };
  if (node.role !== "literal" && node.role !== "case") {
    return undefined;
  }
  if (node.role === "case") {
    const union = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
    const cases = union?.kind === "union" ? (union.cases ?? []).filter((candidate) => (candidate.properties ?? []).length === 0) : [];
    return cases.length > 0 && node.name !== undefined ? { at, key, kind: "case", value: node.name, cases, ...extra } : undefined;
  }
  const written = node.value ?? "";
  if (isHexColor(written)) return { at, key, kind: "color", value: written, colorLiteral: true, ...extra };
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
    const bounded = boundedHints(hints, boundsOf(index, at), declared.base);
    return { at, key, kind: "number", value: written, signed, ...extra, ...(bounded === undefined ? {} : { hints: bounded }) };
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
      const elementAt = attributeElement(tree, node.parent!);
      const element = elementAt === undefined ? undefined : tree.nodes[elementAt];
      return element?.role === "element"
        ? elementProperties(tree, element)?.find((property) => property.name === owner.name)?.type
        : undefined;
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
 * The bounds the constraint of the type the place of the whole value at `at` declares puts on it:
 * its property's, its `let`'s, or for an item, its list's.
 */
function boundsOf(index: TreeIndex, at: number): SourceBounds | undefined {
  const { tree } = index;
  const node = tree.nodes[at]!;
  const owner = node.parent === undefined ? undefined : tree.nodes[node.parent];
  switch (owner?.role) {
    case "attribute": {
      const elementAt = attributeElement(tree, node.parent!);
      const element = elementAt === undefined ? undefined : tree.nodes[elementAt];
      return element?.role === "element"
        ? elementProperties(tree, element)?.find((property) => property.name === owner.name)?.bounds
        : undefined;
    }
    case "declaration":
      return owner.declaration === undefined ? undefined : tree.declarations[owner.declaration]?.bounds;
    case "sequence":
      return boundsOf(index, node.parent!);
    default:
      return undefined;
  }
}

/** `spelled` without its constraint: `float32 where in 0.0..=1.0` is `float32`. */
function withoutConstraint(spelled: string): string {
  return spelled.replace(/\s+where\s[\s\S]*$/, "");
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
  const unconstrained = spelled === undefined ? undefined : withoutConstraint(spelled);
  let name = unconstrained?.replace(/[?+*]+$/, "").trim();
  let list = unconstrained !== undefined && /[+*]/.test(unconstrained.trim().slice(-2));
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
    const target = withoutConstraint(alias.type);
    list ||= /[+*]/.test(target.trim().slice(-2));
    name = target.replace(/[?+*]+$/, "").trim();
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
const BOOLEAN_TYPES = new Set(["boolean"]);

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
  const elementAt = attribute?.role === "attribute" ? attributeElement(tree, place!) : undefined;
  if (attribute !== undefined && elementAt !== undefined) {
    const element = tree.nodes[elementAt]!;
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
  const attributeAt = tree.nodes[at]!.parent;
  const attribute = attributeAt === undefined ? undefined : tree.nodes[attributeAt];
  if (attribute?.role !== "attribute" || attribute.name === undefined || attribute.flags?.includes("content") === true) {
    return undefined;
  }
  const element = attributeElement(tree, attributeAt!);
  return element === undefined ? undefined : { element: tree.nodes[element]!.key, name: attribute.name };
}

/** A property an element does not write, as the reading offers it to add. */
export interface UnwrittenProperty {
  readonly property: SourceProperty;
  /** How its value edits, or nothing when no editor takes its type, as a record's or a list's. */
  readonly editor: TypeEditor | undefined;
  readonly hints: EditorHints | undefined;
  readonly format: SourceValueFormat | undefined;
}

/**
 * The properties the element at `at` declares and does not write, even under a property-list
 * condition, its content left out, in the order its visualizer lists them, else in declaration
 * order. Nothing for a node that is not an element.
 */
export function unwrittenProperties(index: TreeIndex, at: number): UnwrittenProperty[] {
  const { tree } = index;
  const node = tree.nodes[at];
  const declaration = node?.declaration === undefined ? undefined : tree.declarations[node.declaration];
  const declared = node?.role === "element" ? elementProperties(tree, node) : undefined;
  if (declaration === undefined || declared === undefined) {
    return [];
  }
  const written = new Set(
    tree.nodes.flatMap((candidate, position) =>
      candidate.role === "attribute" && attributeElement(tree, position) === at ? [candidate.name ?? ""] : []
    )
  );
  const properties = declared.filter(
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
      const declared = resolveType(index, property.type);
      const format = visualizer?.format ?? declared.format;
      const editor = editorOfType(index, property.type, format);
      const hints = visualizer?.editor ?? declared.editor;
      return {
        property,
        editor,
        hints: editor?.kind === "number" ? boundedHints(hints, property.bounds, declared.base) : hints,
        format
      };
    });
}
