/**
 * Named values: the values a property can take by name, as the source tree's suggestions list them,
 * and the names the reading shows as tokens. A name is a reference to a top-level value, a
 * parameter or a state field, a member chain on one, or an `env.` read such as `env.color.accent`.
 */
import type { SourcePreview, SourceProperty, SourceSuggestion, SourceTree } from "@nx-lang/language-protocol";

import { attributeElement, elementProperties, type TreeIndex } from "./tree.js";

/**
 * The name the node at `at` spells, as `env.space.md` or `brand.accent`: a reference, or a member
 * chain that ends at one. Nothing for any other node, as a call or an operator.
 */
export function nameAt(index: TreeIndex, at: number): string | undefined {
  const node = index.tree.nodes[at];
  if (node === undefined || node.name === undefined) {
    return undefined;
  }
  if (node.role === "reference") {
    return node.name;
  }
  if (node.role !== "member") {
    return undefined;
  }
  const object = index.byKey.get(`${node.key}.object`);
  const base = object === undefined ? undefined : nameAt(index, object);
  return base === undefined ? undefined : `${base}.${node.name}`;
}

/** The reference a name at `at` starts from: itself, or the end of its member chain. */
export function nameRoot(index: TreeIndex, at: number): number | undefined {
  const node = index.tree.nodes[at];
  if (node?.role === "reference") {
    return at;
  }
  if (node?.role !== "member") {
    return undefined;
  }
  const object = index.byKey.get(`${node.key}.object`);
  return object === undefined ? undefined : nameRoot(index, object);
}

/** The index in `declarations` of the top-level declaration the node at `at` is written in. */
export function enclosingDeclaration(tree: SourceTree, at: number): number | undefined {
  let current: number | undefined = at;
  while (current !== undefined && tree.nodes[current]!.parent !== undefined) {
    current = tree.nodes[current]!.parent;
  }
  const root = current === undefined ? undefined : tree.nodes[current];
  return root?.role === "declaration" ? root.declaration : undefined;
}

/** A place that can take a named value: a property an element writes or leaves out, or a `let`'s value. */
export interface NamedPlace {
  /** The key of the place's row: the attribute's, the `let`'s, or, for a property left out, `{element}.{name}`. */
  readonly key: string;
  /** The suggestion key of its type. */
  readonly suggestionKey: string;
  /** The index in `declarations` of the top-level declaration it is written in. */
  readonly within: number | undefined;
  /** The index of its value's node, when written. */
  readonly value?: number;
  /** For a property, the element's key and the property's name. */
  readonly property?: { readonly element: string; readonly name: string };
  /** Set for a property the element does not write, which choosing adds. */
  readonly adding: boolean;
  /** The name its value is now, when it is one. */
  readonly current?: string;
  /** Its property's default, as written, when it has one. */
  readonly default?: string;
  /** The property's type, as spelled. */
  readonly type?: string;
}

/** The node kinds a named value may be written over: a literal, a case, a name, an element, or a color call. */
function replaceable(index: TreeIndex, at: number): boolean {
  const node = index.tree.nodes[at]!;
  switch (node.role) {
    case "literal":
    case "case":
      return true;
    case "reference":
    case "member":
      return nameAt(index, at) !== undefined;
    case "element":
      return true;
    case "call":
      return node.color !== undefined;
    default:
      return false;
  }
}

/**
 * The place the attribute at `at` is, when its property's type has a suggestion key and its value
 * is one a name may be written over.
 */
export function attributePlace(index: TreeIndex, at: number): NamedPlace | undefined {
  const { tree } = index;
  const attribute = tree.nodes[at];
  if (attribute?.role !== "attribute" || attribute.name === undefined || attribute.flags?.includes("content") === true) {
    return undefined;
  }
  const elementAt = attributeElement(tree, at);
  const element = elementAt === undefined ? undefined : tree.nodes[elementAt]!;
  const property = element === undefined ? undefined : elementProperties(tree, element)?.find((candidate) => candidate.name === attribute.name);
  const value = index.byKey.get(`${attribute.key}.value`);
  if (property?.suggestionKey === undefined || element === undefined || value === undefined || !replaceable(index, value)) {
    return undefined;
  }
  const current = nameAt(index, value);
  return {
    key: attribute.key,
    suggestionKey: property.suggestionKey,
    within: enclosingDeclaration(tree, at),
    value,
    property: { element: element.key, name: attribute.name },
    adding: false,
    ...(current === undefined ? {} : { current }),
    ...(property.default === undefined ? {} : { default: property.default }),
    type: property.type
  };
}

/** The place of `property`, which the element at `elementAt` does not write. */
export function unsetPlace(index: TreeIndex, elementAt: number, property: SourceProperty): NamedPlace | undefined {
  const { tree } = index;
  const element = tree.nodes[elementAt]!;
  if (property.suggestionKey === undefined) {
    return undefined;
  }
  return {
    key: `${element.key}.${property.name}`,
    suggestionKey: property.suggestionKey,
    within: enclosingDeclaration(tree, elementAt),
    property: { element: element.key, name: property.name },
    adding: true,
    ...(property.default === undefined ? {} : { default: property.default }),
    type: property.type
  };
}

/** The place of the value of the parameterless top-level `let` at `at`, when its type has a key. */
export function letPlace(index: TreeIndex, at: number): NamedPlace | undefined {
  const { tree } = index;
  const node = tree.nodes[at];
  if (node?.role !== "declaration" || node.parent !== undefined || node.declaration === undefined) {
    return undefined;
  }
  const declaration = tree.declarations[node.declaration];
  const value = index.byKey.get(`${node.key}.value`);
  if (declaration?.kind !== "value" || declaration.suggestionKey === undefined || value === undefined || !replaceable(index, value)) {
    return undefined;
  }
  const current = nameAt(index, value);
  return {
    key: node.key,
    suggestionKey: declaration.suggestionKey,
    within: node.declaration,
    value,
    adding: false,
    ...(current === undefined ? {} : { current }),
    ...(declaration.type === undefined ? {} : { type: declaration.type })
  };
}

/**
 * The named values `place` offers, in the tree's order: those with its key, leaving out a parameter
 * or state field of another declaration and a value marked `self` with the place's declaration.
 */
export function namedValues(tree: SourceTree, place: NamedPlace): SourceSuggestion[] {
  return (tree.suggestions ?? []).filter((suggestion) => suggestion.key === place.suggestionKey && reaches(suggestion, place.within));
}

/** Whether `suggestion` reaches a place written in the declaration at index `within`. */
function reaches(suggestion: SourceSuggestion, within: number | undefined): boolean {
  return (
    (suggestion.scope === undefined || suggestion.scope === within) &&
    !(within !== undefined && suggestion.self?.includes(within) === true)
  );
}

/**
 * The suggestion the tree has for `name` as written in the declaration at index `within`: one that
 * reaches there, as {@link namedValues} filters them, preferring one with `key`, then one with a
 * preview.
 */
export function suggestionNamed(tree: SourceTree, name: string, within: number | undefined, key?: string): SourceSuggestion | undefined {
  const all = (tree.suggestions ?? []).filter((suggestion) => suggestion.name === name && reaches(suggestion, within));
  const withPreview = all.filter((suggestion) => suggestion.preview !== undefined);
  return withPreview.find((suggestion) => suggestion.key === key) ?? withPreview[0] ?? all[0];
}

/**
 * The text a preview reads as: a color's hex, or, when it has none, its CSS with its numbers cut
 * to three decimals and an opaque alpha left out; a string quoted; a number, boolean or case as
 * written.
 */
export function previewText(preview: SourcePreview): string {
  switch (preview.kind) {
    case "color":
      return (
        preview.hex ??
        preview.value
          .replace(/-?\d+\.\d{4,}/g, (number) => String(Number(Number(number).toFixed(3))))
          .replace(/\s*\/\s*1\)$/, ")")
      );
    case "string":
      return JSON.stringify(preview.value);
    default:
      return preview.value;
  }
}

/**
 * The words of a filter, lowercased, `.` taken as a space: a name holds them all when its own
 * words, read the same way, hold each.
 */
export function filterWords(typed: string): string[] {
  return typed.toLowerCase().replace(/\./g, " ").split(/\s+/).filter((word) => word !== "");
}

/**
 * Whether a row holds each of `words`: in its name anywhere, `.` taken as a space, or at
 * the start of a word of its doc, so prose does not match inside its words.
 */
export function rowHolds(name: string, doc: string | undefined, words: readonly string[]): boolean {
  const spelled = name.toLowerCase().replace(/\./g, " ");
  const docWords = (doc ?? "").toLowerCase().split(/[^\p{L}\p{N}]+/u).filter((word) => word !== "");
  return words.every((word) => spelled.includes(word) || docWords.some((docWord) => docWord.startsWith(word)));
}

/**
 * `whole` cut to fit, as `fits` reports for each try: whole middle parts dropped first, keeping the
 * first part and as many last parts as fit (`color.….muted`), then just the last parts
 * (`….muted`), then characters from the middle of what is left, keeping more of the end.
 */
export function cutName(whole: string, fits: (tried: string) => boolean): string {
  const parts = whole.split(".");
  for (let keep = parts.length - 2; keep >= 1; keep--) {
    const tried = `${parts[0]}.\u2026.${parts.slice(parts.length - keep).join(".")}`;
    if (fits(tried)) {
      return tried;
    }
  }
  if (parts.length > 1 && fits(`\u2026.${parts[parts.length - 1]}`)) {
    return `\u2026.${parts[parts.length - 1]}`;
  }
  let low = 1;
  let high = whole.length - 1;
  let best = `\u2026${whole.slice(-1)}`;
  while (low <= high) {
    const keep = Math.floor((low + high) / 2);
    const tail = Math.ceil(keep * 0.6);
    const tried = `${whole.slice(0, keep - tail)}\u2026${whole.slice(whole.length - tail)}`;
    if (fits(tried)) {
      best = tried;
      low = keep + 1;
    } else {
      high = keep - 1;
    }
  }
  return best;
}

/** The aliases the document's imports give, as `d` for `import "@nx/design" as d`. */
export function importAliases(tree: SourceTree): Set<string> {
  return new Set(tree.nodes.flatMap((node) => (node.role === "import" && node.parent === undefined && node.name !== undefined ? [node.name] : [])));
}

/**
 * The part of a suggestion's name that reads quieter, its leading `env.` or the import alias it is
 * spelled through, and the rest.
 */
export function nameParts(name: string, aliases: ReadonlySet<string>): { readonly quiet: string; readonly rest: string } {
  const dot = name.indexOf(".");
  const first = dot < 0 ? "" : name.slice(0, dot);
  return first === "env" || aliases.has(first) ? { quiet: `${first}.`, rest: name.slice(dot + 1) } : { quiet: "", rest: name };
}
