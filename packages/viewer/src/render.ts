/**
 * The reading view: one renderer for each role of the source tree. Each turns a node and its
 * children into DOM and marks the result with the node's key, so the reading is lossless by
 * construction: a node no renderer draws is missing from the rendering, which the coverage test
 * catches. Only the element renderer consults the declaration table, for property order and
 * defaults.
 */
import type { SourceNode, SourceProperty, SourceRole } from "@nx-lang/language-protocol";

import { renderText, type TextPiece } from "./text.js";
import { childrenIn, declarationOf, nodeOf, type TreeIndex } from "./tree.js";
import {
  BINARY_READINGS,
  eventWords,
  inWords,
  KIND_LABELS,
  POSTFIX_READINGS,
  PREFIX_READINGS,
} from "./words.js";

/** What every renderer is given. */
export interface RenderContext {
  readonly document: Document;
  readonly index: TreeIndex;
  /** Whether omitted defaults show as ghost rows. */
  readonly ghosts: boolean;
  /**
   * Whether this is a copy of nodes rendered elsewhere, as a reference's expansion is: a copy
   * carries each node's key as `data-ref-key`, so the original stays the one node with the key.
   */
  readonly copy: boolean;
}

/** Turns the node at `at` into DOM, or nothing. */
export type Renderer = (context: RenderContext, at: number, options: RenderOptions) => HTMLElement;

/** What a parent tells the renderer of a child. */
export interface RenderOptions {
  /** The name of the declaration whose value this element is, shown on its card. */
  readonly declarationName?: string;
  /** Whether a string literal reads as words of a sentence, with no value style. */
  readonly inSentence?: boolean;
}

/** Renders the node at `at` by its role; nothing when no renderer draws the role. */
export function renderNode(context: RenderContext, at: number, options: RenderOptions = {}): HTMLElement | undefined {
  const renderer = renderers[nodeOf(context.index, at).role];
  return renderer?.(context, at, options);
}

/** Renders the whole document: its top-level nodes in order, with its imports in one strip. */
export function renderDocument(context: RenderContext): HTMLElement {
  const { document, index } = context;
  const root = document.createElement("div");
  root.className = "document";
  let imports: HTMLElement | undefined;
  index.tree.nodes.forEach((node, at) => {
    if (node.parent !== undefined) {
      return;
    }
    if (node.role === "import") {
      if (imports === undefined) {
        imports = detailsStrip(document, "Imports");
        root.append(imports);
      }
      appendNode(context, imports.querySelector(".details-body")!, at);
      const summary = imports.querySelector("summary")!;
      const count = imports.querySelectorAll(":scope > .details-body > [data-role='import']").length;
      summary.textContent = `Imports · ${count}`;
      return;
    }
    appendNode(context, root, at);
  });
  return root;
}

function detailsStrip(document: Document, title: string): HTMLElement {
  const details = document.createElement("details");
  details.className = "details";
  const summary = document.createElement("summary");
  summary.textContent = title;
  const body = document.createElement("div");
  body.className = "details-body";
  details.append(summary, body);
  return details;
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

function node(context: RenderContext, at: number): SourceNode {
  return nodeOf(context.index, at);
}

function has(node: SourceNode, flag: NonNullable<SourceNode["flags"]>[number]): boolean {
  return node.flags?.includes(flag) ?? false;
}

/** An element marked as the rendering of the node at `at`. */
function marked<K extends keyof HTMLElementTagNameMap>(
  context: RenderContext,
  at: number,
  tag: K,
  className: string
): HTMLElementTagNameMap[K] {
  const element = context.document.createElement(tag);
  const source = node(context, at);
  element.className = `n ${className}`;
  element.dataset["role"] = source.role;
  if (context.copy) {
    element.dataset["refKey"] = source.key;
  } else {
    element.dataset["key"] = source.key;
  }
  return element;
}

function span(document: Document, className: string, text: string): HTMLSpanElement {
  const element = document.createElement("span");
  element.className = className;
  element.textContent = text;
  return element;
}

function appendNode(context: RenderContext, parent: HTMLElement, at: number, options: RenderOptions = {}): void {
  const rendered = renderNode(context, at, options);
  if (rendered !== undefined) {
    parent.append(rendered);
  }
}

function appendAll(context: RenderContext, parent: HTMLElement, children: readonly number[], options: RenderOptions = {}): void {
  for (const child of children) {
    appendNode(context, parent, child, options);
  }
}

/** Appends `children` separated by `separator`, comments included as they stand. */
function appendJoined(context: RenderContext, parent: HTMLElement, children: readonly number[], separator: string): void {
  children.forEach((child, position) => {
    if (position > 0 && !isNote(node(context, child))) {
      parent.append(separator);
    }
    appendNode(context, parent, child);
  });
}

function isNote(source: SourceNode): boolean {
  return source.role === "comment" || source.role === "docComment";
}

/** The "Source" button a card or a declaration carries in its header. */
function sourceButton(document: Document, key: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "source-toggle";
  button.textContent = "Source";
  button.dataset["sourceFor"] = key;
  button.setAttribute("aria-label", "Show the source");
  return button;
}

function label(document: Document, text: string): HTMLSpanElement {
  return span(document, "group-label", text);
}

function isBlockRole(role: SourceRole): boolean {
  return (
    role === "element" ||
    role === "condition" ||
    role === "match" ||
    role === "loop" ||
    role === "declaration" ||
    role === "comment" ||
    role === "docComment" ||
    role === "unparsed"
  );
}

// ---------------------------------------------------------------------------------------------
// Declarations and their members
// ---------------------------------------------------------------------------------------------

const MEMBER_ROLES: ReadonlySet<SourceRole> = new Set(["parameter", "stateField", "field", "unionCase", "emit"]);

const renderDeclaration: Renderer = (context, at) => {
  const { document, index } = context;
  const source = node(context, at);
  const entry = declarationOf(index, source);
  const children = index.children[at] ?? [];
  const valueNodes = children.filter(
    (child) => !MEMBER_ROLES.has(node(context, child).role) && index.slots[child] !== "type" && !isNote(node(context, child))
  );

  // `let roleQuestion = <SingleChoice … />` reads as the card, named by the declaration.
  const members = children.filter((child) => MEMBER_ROLES.has(node(context, child).role));
  const onlyValue = valueNodes.length === 1 ? valueNodes[0]! : undefined;
  if (
    members.length === 0 &&
    onlyValue !== undefined &&
    node(context, onlyValue).role === "element" &&
    children.every((child) => child === onlyValue || isNote(node(context, child)))
  ) {
    const section = marked(context, at, "section", "declaration named-value");
    for (const child of children) {
      appendNode(context, section, child, child === onlyValue ? { declarationName: source.name ?? "" } : {});
    }
    return section;
  }

  const section = marked(context, at, "section", "declaration");
  const header = document.createElement("div");
  header.className = "declaration-header";
  for (const flag of source.flags ?? []) {
    header.append(span(document, "tag", flag));
  }
  if (entry !== undefined) {
    header.append(span(document, "declaration-kind", KIND_LABELS[entry.kind]));
  }
  header.append(span(document, "declaration-name", source.name ?? ""));
  if (!context.copy) {
    header.append(sourceButton(document, source.key));
  }
  section.append(header);

  const body = document.createElement("div");
  body.className = "declaration-body";
  let group: string | undefined;
  for (const child of children) {
    const childNode = node(context, child);
    const slot = index.slots[child];
    let groupName: string | undefined;
    if (MEMBER_ROLES.has(childNode.role)) {
      groupName = memberGroup(childNode.role, entry?.kind);
    } else if (slot === "type") {
      groupName = entry?.kind === "alias" ? "Is" : "Gives";
    } else if (!isNote(childNode)) {
      groupName = slot === "body" ? (entry?.kind === "component" ? "Shows" : "Gives") : "Is";
    }
    if (groupName !== undefined && groupName !== group) {
      body.append(label(document, groupName));
      group = groupName;
    }
    appendNode(context, body, child);
  }
  section.append(body);
  return section;
};

function memberGroup(role: SourceRole, kind: string | undefined): string {
  switch (role) {
    case "parameter":
      return kind === "component" ? "Properties" : "Takes";
    case "stateField":
      return "State";
    case "emit":
      return "Emits";
    case "unionCase":
      return "Cases";
    default:
      return "Fields";
  }
}

/** A field, a parameter or a state field: its name, its type, and its default. */
const renderMember: Renderer = (context, at) => {
  const { document, index } = context;
  const source = node(context, at);
  const row = marked(context, at, "div", "member");
  // Spaces between the parts, so the row reads as words to a screen reader and when copied.
  row.append(span(document, "member-name", source.name ?? ""));
  if (has(source, "optional")) {
    row.append(" ", span(document, "tag", "optional"));
  }
  if (has(source, "content")) {
    row.append(" ", span(document, "tag", "content"));
  }
  for (const child of index.children[at] ?? []) {
    if (index.slots[child] === "default") {
      row.append(" ", span(document, "member-default-label", "default"));
    }
    row.append(" ");
    appendNode(context, row, child);
  }
  return row;
};

/** A case of a union, with the fields it carries. */
const renderUnionCase: Renderer = (context, at) => {
  const { document, index } = context;
  const row = marked(context, at, "div", "member union-case");
  row.append(span(document, "pill", node(context, at).name ?? ""));
  const fields = document.createElement("div");
  fields.className = "nested";
  appendAll(context, fields, index.children[at] ?? []);
  if (fields.childElementCount > 0) {
    row.append(fields);
  }
  return row;
};

/** An action a component emits, with what it extends and the fields it carries. */
const renderEmit: Renderer = (context, at) => {
  const { document, index } = context;
  const row = marked(context, at, "div", "member emit");
  row.append(span(document, "member-name", node(context, at).name ?? ""));
  const fields = document.createElement("div");
  fields.className = "nested";
  for (const child of index.children[at] ?? []) {
    if (index.slots[child] === "extends") {
      row.append(span(document, "member-default-label", "extends"));
      appendNode(context, row, child);
    } else {
      appendNode(context, fields, child);
    }
  }
  if (fields.childElementCount > 0) {
    row.append(fields);
  }
  return row;
};

const renderTypeReference: Renderer = (context, at) => {
  const element = marked(context, at, "code", "type");
  element.textContent = node(context, at).value ?? "";
  return element;
};

const renderImport: Renderer = (context, at) => {
  const { document, index } = context;
  const row = marked(context, at, "div", "import");
  const children = index.children[at] ?? [];
  const names = children.filter((child) => index.slots[child] !== "path");
  const path = children.filter((child) => index.slots[child] === "path");
  if (names.length > 0) {
    row.append("Uses ");
    appendJoined(context, row, names, ", ");
    row.append(" from ");
  } else {
    row.append("Uses ");
  }
  appendAll(context, row, path);
  return row;
};

// ---------------------------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------------------------

/** The attributes an element sets anywhere in its tag, conditional ones included. */
function attributesSetBy(context: RenderContext, element: number): Set<string> {
  const names = new Set<string>();
  const walk = (at: number) => {
    for (const child of context.index.children[at] ?? []) {
      const childNode = node(context, child);
      if (childNode.role === "attribute") {
        names.add(childNode.name ?? "");
      } else if (childNode.role !== "element") {
        walk(child);
      }
    }
  };
  walk(element);
  return names;
}

/** Groups each comment with the node it precedes, so reordering keeps a comment with its node. */
function units(context: RenderContext, children: readonly number[]): number[][] {
  const grouped: number[][] = [];
  let pending: number[] = [];
  for (const child of children) {
    pending.push(child);
    if (!isNote(node(context, child))) {
      grouped.push(pending);
      pending = [];
    }
  }
  if (pending.length > 0) {
    grouped.push(pending);
  }
  return grouped;
}

function lastOf(unit: readonly number[]): number {
  return unit[unit.length - 1]!;
}

const renderElement: Renderer = (context, at, options) => {
  const { document, index } = context;
  const source = node(context, at);
  if (has(source, "stateUpdate")) {
    return renderStateUpdate(context, at);
  }
  const entry = declarationOf(index, source);
  const card = marked(context, at, "div", "card");
  card.tabIndex = 0;

  const header = document.createElement("div");
  header.className = "card-header";
  const kind = span(document, entry === undefined ? "kind unresolved" : "kind", inWords(source.name ?? ""));
  if (entry === undefined) {
    kind.textContent = source.name ?? "";
    kind.title = "This kind is not declared anywhere the document can see.";
    header.append(kind, span(document, "unresolved-marker", "unresolved"));
  } else {
    header.append(kind);
  }
  if (options.declarationName !== undefined) {
    header.append(span(document, "card-name", options.declarationName));
  }
  if (source.textType !== undefined && source.textType !== "") {
    header.append(span(document, "tag", source.textType));
  }
  if (!context.copy) {
    header.append(sourceButton(document, source.key));
  }
  card.append(header);

  const children = index.children[at] ?? [];
  const grouped = units(context, children);
  const attributeUnits = grouped.filter((unit) => node(context, lastOf(unit)).role === "attribute");
  const otherUnits = grouped.filter((unit) => node(context, lastOf(unit)).role !== "attribute");

  const rows = document.createElement("div");
  rows.className = "rows";
  const properties: readonly SourceProperty[] = entry?.properties ?? [];
  const byName = new Map(attributeUnits.map((unit) => [node(context, lastOf(unit)).name ?? "", unit]));
  const placed = new Set<number[]>();
  const set = attributesSetBy(context, at);
  for (const property of properties) {
    const unit = byName.get(property.name);
    if (unit !== undefined && !placed.has(unit)) {
      appendAll(context, rows, unit);
      placed.add(unit);
    } else if (context.ghosts && property.default !== undefined && !set.has(property.name)) {
      rows.append(ghostRow(document, property));
    }
  }
  for (const unit of attributeUnits) {
    if (!placed.has(unit)) {
      appendAll(context, rows, unit);
    }
  }
  if (rows.childElementCount > 0) {
    card.append(rows);
  }

  // What the element holds: text, embeds, nested elements and logic, in source order.
  const content = document.createElement("div");
  content.className = "content";
  appendContent(context, content, otherUnits.flat(), source.textType === "markdown");
  if (context.copy) {
    // A copy's text runs carry their keys as references to the originals.
    for (const run of content.querySelectorAll<HTMLElement>("[data-key]")) {
      run.dataset["refKey"] = run.dataset["key"]!;
      delete run.dataset["key"];
    }
  }
  if (content.childElementCount > 0) {
    card.append(content);
  }
  return card;
};

/**
 * Appends an element's content: runs of text and embeds as one body of prose, markdown when the
 * element's text type is, and everything else as it renders, in source order.
 */
function appendContent(context: RenderContext, container: HTMLElement, children: readonly number[], markdown: boolean): void {
  const { document } = context;
  let pieces: TextPiece[] = [];
  const flushText = () => {
    if (pieces.length > 0) {
      container.append(renderText(document, pieces, markdown));
      pieces = [];
    }
  };
  for (const child of children) {
    const childNode = node(context, child);
    if (childNode.role === "text") {
      pieces.push({ kind: "text", key: childNode.key, value: childNode.value ?? "", raw: has(childNode, "raw") });
    } else if (childNode.role === "embed") {
      const embed = renderNode(context, child);
      if (embed !== undefined) {
        pieces.push({ kind: "embed", element: embed });
      }
    } else {
      flushText();
      appendNode(context, container, child);
    }
  }
  flushText();
}

/** The text type of the element an attribute belongs to. */
function textTypeOf(context: RenderContext, attribute: number): string | undefined {
  for (let at = node(context, attribute).parent; at !== undefined; at = node(context, at).parent) {
    const ancestor = node(context, at);
    if (ancestor.role === "element") {
      return ancestor.textType;
    }
  }
  return undefined;
}

function ghostRow(document: Document, property: SourceProperty): HTMLElement {
  const row = document.createElement("div");
  row.className = "row ghost";
  const name = span(document, "label", property.name);
  name.dataset["property"] = property.name;
  name.tabIndex = 0;
  const value = document.createElement("span");
  value.className = "row-value";
  value.append(span(document, "ghost-value", property.default ?? ""), span(document, "tag", "default"));
  row.append(name, value);
  return row;
}

/** `<Update a={…} b={…} />` in a component: one "set a to …" line per attribute. */
function renderStateUpdate(context: RenderContext, at: number): HTMLElement {
  const { document, index } = context;
  const block = marked(context, at, "div", "update");
  for (const child of index.children[at] ?? []) {
    const childNode = node(context, child);
    if (childNode.role !== "attribute") {
      appendNode(context, block, child);
      continue;
    }
    const line = marked(context, child, "div", "set-line");
    line.append(span(document, "verb", "set "), span(document, "slot-name", childNode.name ?? ""), span(document, "verb", " to "));
    appendAll(context, line, index.children[child] ?? []);
    block.append(line);
  }
  return block;
}

const renderAttribute: Renderer = (context, at) => {
  const { document, index } = context;
  const source = node(context, at);
  const row = marked(context, at, "div", "row");
  const children = index.children[at] ?? [];

  if (has(source, "handler")) {
    row.classList.add("handler");
    row.append(span(document, "when", `When ${eventWords(source.name ?? "")}`));
    const value = document.createElement("div");
    value.className = "row-value";
    const values = children.filter((child) => !isNote(node(context, child)));
    const update = values.length === 1 && has(node(context, values[0]!), "stateUpdate");
    if (!update) {
      value.append(span(document, "arrow", "→ "));
    }
    appendAll(context, value, children);
    row.append(value);
    return row;
  }

  const name = span(document, "label", source.name ?? "");
  name.dataset["property"] = source.name ?? "";
  name.tabIndex = 0;
  const value = document.createElement("div");
  value.className = has(source, "content") ? "row-value items" : "row-value";
  if (has(source, "content")) {
    row.classList.add("content-row");
    const list = document.createElement("div");
    list.className = "item-list";
    appendContent(context, list, children, textTypeOf(context, at) === "markdown");
    value.append(list);
  } else {
    appendAll(context, value, children);
  }
  row.append(name, value);
  return row;
};

const renderTextRun: Renderer = (context, at) => {
  const source = node(context, at);
  const element = renderText(
    context.document,
    [{ kind: "text", key: source.key, value: source.value ?? "", raw: has(source, "raw") }],
    false
  );
  if (context.copy) {
    for (const run of element.querySelectorAll<HTMLElement>("[data-key]")) {
      run.dataset["refKey"] = run.dataset["key"]!;
      delete run.dataset["key"];
    }
  }
  return element;
};

const renderEmbed: Renderer = (context, at) => {
  const element = marked(context, at, "span", "slot embed");
  appendAll(context, element, context.index.children[at] ?? []);
  return element;
};

// ---------------------------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------------------------

/** A string literal's text without its quotes. */
export function unquote(literal: string): string {
  if (!literal.startsWith('"')) {
    return literal;
  }
  try {
    return JSON.parse(literal) as string;
  } catch {
    return literal.slice(1, literal.endsWith('"') && literal.length > 1 ? -1 : undefined);
  }
}

const renderLiteral: Renderer = (context, at, options) => {
  const source = node(context, at);
  const text = source.value ?? "";
  if (text.startsWith('"')) {
    const element = marked(context, at, "span", options.inSentence === true ? "words" : "value string");
    element.textContent = unquote(text);
    return element;
  }
  if (text === "true" || text === "false") {
    const element = marked(context, at, "span", `value boolean ${text}`);
    element.textContent = text === "true" ? "✓" : "✗";
    element.setAttribute("role", "img");
    element.setAttribute("aria-label", text);
    element.title = text;
    return element;
  }
  const element = marked(context, at, "span", "value number");
  element.textContent = text;
  return element;
};

const renderEmpty: Renderer = (context, at) => {
  const element = marked(context, at, "span", "value empty");
  element.textContent = "empty";
  return element;
};

const renderCase: Renderer = (context, at) => {
  const { document, index } = context;
  const element = marked(context, at, "span", "pill case");
  const qualifier = index.children[at] ?? [];
  if (qualifier.length > 0) {
    const muted = span(document, "qualifier", "");
    appendAll(context, muted, qualifier);
    element.append(muted, " ");
  }
  element.append(node(context, at).name ?? "");
  return element;
};

const renderSequence: Renderer = (context, at) => {
  const { index } = context;
  const children = index.children[at] ?? [];
  const blocks = children.some((child) => isBlockRole(node(context, child).role));
  const element = marked(context, at, "div", blocks ? "sequence blocks" : "sequence");
  appendAll(context, element, children);
  return element;
};

// ---------------------------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------------------------

const renderReference: Renderer = (context, at) => {
  const { document, index } = context;
  const source = node(context, at);
  const wrapper = marked(context, at, "span", "reference");
  const name = span(document, "ref", source.name ?? "");
  name.tabIndex = 0;
  wrapper.append(name);
  const entry = declarationOf(index, source);
  if (entry?.valueRange !== undefined && !context.copy && valueNodeOf(context, entry.valueRange) !== undefined) {
    const expand = document.createElement("button");
    expand.type = "button";
    expand.className = "expand";
    expand.textContent = "▸";
    expand.setAttribute("aria-expanded", "false");
    expand.setAttribute("aria-label", `Show ${source.name ?? "the value"} here`);
    wrapper.append(expand);
  }
  return wrapper;
};

/** The node whose range is a value declaration's value range, for expanding a reference to it. */
export function valueNodeOf(context: RenderContext, range: { startByte: number; endByte: number }): number | undefined {
  const nodes = context.index.tree.nodes;
  let found: number | undefined;
  nodes.forEach((candidate, at) => {
    if (found === undefined && candidate.range.startByte === range.startByte && candidate.range.endByte === range.endByte) {
      found = at;
    }
  });
  return found;
}

const renderMemberAccess: Renderer = (context, at) => {
  const { document } = context;
  const source = node(context, at);
  const element = marked(context, at, "span", "member-access");
  appendAll(context, element, context.index.children[at] ?? []);
  element.append(span(document, "dot", has(source, "optional") ? "?." : "."), span(document, "member-name", source.name ?? ""));
  return element;
};

const renderCall: Renderer = (context, at) => {
  const { index } = context;
  const element = marked(context, at, "span", "call");
  appendAll(context, element, childrenIn(index, at, "callee"));
  element.append("(");
  appendJoined(
    context,
    element,
    (index.children[at] ?? []).filter((child) => index.slots[child] !== "callee"),
    ", "
  );
  element.append(")");
  return element;
};

/** Whether the node is a string `+`, which reads as a sentence. */
function isSentence(source: SourceNode): boolean {
  return source.role === "operator" && source.value === "+" && source.type === "string";
}

const renderOperator: Renderer = (context, at) => {
  const { document, index } = context;
  const source = node(context, at);
  const token = source.value ?? "";
  const children = index.children[at] ?? [];
  const element = marked(context, at, "span", "operator");
  const parenthesized = has(source, "parenthesized");
  if (parenthesized) {
    element.append("(");
  }

  if (isSentence(source)) {
    element.classList.add("sentence");
    for (const child of children) {
      const childNode = node(context, child);
      if (childNode.role === "literal" && (childNode.value ?? "").startsWith('"')) {
        appendNode(context, element, child, { inSentence: true });
      } else if (isSentence(childNode) && !has(childNode, "parenthesized")) {
        appendNode(context, element, child, { inSentence: true });
      } else if (isNote(childNode)) {
        appendNode(context, element, child);
      } else {
        const slot = span(document, "slot", "");
        appendNode(context, slot, child);
        element.append(slot);
      }
    }
  } else {
    const left = childrenIn(index, at, "left");
    const right = childrenIn(index, at, "right");
    const operand = childrenIn(index, at, "operand");
    const placed = [...left, ...right, ...operand];
    const binary = BINARY_READINGS.get(token);
    if (left.length > 0 && right.length > 0) {
      if (binary?.kind === "around") {
        element.append(span(document, "op", `${binary.before} `));
        appendAll(context, element, left);
        element.append(span(document, "op", ` ${binary.between} `));
        appendAll(context, element, right);
      } else {
        const symbol = binary?.symbol ?? token;
        appendAll(context, element, left);
        element.append(span(document, "op", symbol.startsWith(",") ? `${symbol} ` : ` ${symbol} `));
        appendAll(context, element, right);
      }
    } else if (operand.length > 0 && POSTFIX_READINGS.has(token)) {
      appendAll(context, element, operand);
      element.append(span(document, "op", ` ${POSTFIX_READINGS.get(token)!}`));
    } else if (operand.length > 0) {
      const reading = PREFIX_READINGS.get(token) ?? token;
      element.append(span(document, "op", reading.length > 1 ? `${reading} ` : reading));
      appendAll(context, element, operand);
    } else {
      // A shape the list does not know: its token as written, between its operands.
      placed.push(...children);
      appendJoined(context, element, children, ` ${token} `);
    }
    // Comments inside the operator, after what it reads.
    appendAll(context, element, children.filter((child) => !placed.includes(child)));
  }

  if (parenthesized) {
    element.append(")");
  }
  return element;
};

const renderCondition: Renderer = (context, at) => {
  const { document, index } = context;
  const element = marked(context, at, "div", "condition");
  const test = childrenIn(index, at, "test");
  const arms = (index.children[at] ?? []).filter((child) => node(context, child).role === "matchArm");
  if (test.length > 0) {
    const when = document.createElement("div");
    when.className = "when-label";
    when.append(span(document, "verb", "Only when "));
    appendAll(context, when, test);
    element.append(when);
  }
  if (arms.length > 0) {
    element.classList.add("arms");
  }
  let branch: string | undefined;
  let container: HTMLElement | undefined;
  for (const child of index.children[at] ?? []) {
    if (test.includes(child)) {
      continue;
    }
    const childNode = node(context, child);
    if (childNode.role === "matchArm") {
      appendNode(context, element, child);
      container = undefined;
      branch = undefined;
      continue;
    }
    const slot = index.slots[child] === "else" ? "else" : "then";
    if (slot !== branch || container === undefined) {
      if (slot === "else") {
        element.append(span(document, "otherwise", "Otherwise"));
      }
      container = document.createElement("div");
      container.className = `branch ${slot}`;
      element.append(container);
      branch = slot;
    }
    appendNode(context, container, child);
  }
  return element;
};

const renderMatch: Renderer = (context, at) => {
  const { document, index } = context;
  const element = marked(context, at, "div", "match");
  const subject = childrenIn(index, at, "subject");
  const head = document.createElement("div");
  head.className = "when-label";
  appendAll(context, head, subject);
  head.append(span(document, "verb", " is"));
  element.append(head);
  let otherwise: HTMLElement | undefined;
  for (const child of index.children[at] ?? []) {
    if (subject.includes(child)) {
      continue;
    }
    if (index.slots[child] === "else") {
      if (otherwise === undefined) {
        const row = document.createElement("div");
        row.className = "arm else-arm";
        row.append(span(document, "arm-when", "anything else"));
        otherwise = document.createElement("div");
        otherwise.className = "arm-then";
        row.append(otherwise);
        element.append(row);
      }
      appendNode(context, otherwise, child);
    } else {
      appendNode(context, element, child);
    }
  }
  return element;
};

const renderMatchArm: Renderer = (context, at) => {
  const { document, index } = context;
  const element = marked(context, at, "div", "arm");
  const when = document.createElement("div");
  when.className = "arm-when";
  const then = document.createElement("div");
  then.className = "arm-then";
  const test = childrenIn(index, at, "test");
  const patterns = childrenIn(index, at, "pattern");
  if (test.length > 0) {
    when.append(span(document, "verb", "When "));
    appendAll(context, when, test);
  } else {
    appendJoined(context, when, patterns, " or ");
  }
  appendAll(
    context,
    then,
    (index.children[at] ?? []).filter((child) => !test.includes(child) && !patterns.includes(child))
  );
  element.append(when, then);
  return element;
};

const renderLoop: Renderer = (context, at) => {
  const { document, index } = context;
  const element = marked(context, at, "div", "loop");
  const head = document.createElement("div");
  head.className = "when-label";
  const bindings = (index.children[at] ?? []).filter((child) => node(context, child).role === "binding");
  const iterable = childrenIn(index, at, "in");
  head.append(span(document, "verb", "For each "));
  appendJoined(context, head, bindings, ", ");
  head.append(span(document, "verb", " in "));
  appendAll(context, head, iterable);
  element.append(head);
  const body = document.createElement("div");
  body.className = "branch";
  appendAll(
    context,
    body,
    (index.children[at] ?? []).filter((child) => !bindings.includes(child) && !iterable.includes(child))
  );
  element.append(body);
  return element;
};

const renderBinding: Renderer = (context, at) => {
  const element = marked(context, at, "span", "binding");
  element.textContent = node(context, at).name ?? "";
  return element;
};

// ---------------------------------------------------------------------------------------------
// Comments and what could not be read
// ---------------------------------------------------------------------------------------------

/** A comment's text without its markers. */
export function commentText(comment: string): string {
  if (comment.startsWith("///")) {
    return comment.slice(3).replace(/^ /, "");
  }
  if (comment.startsWith("//")) {
    return comment.slice(2).replace(/^ /, "");
  }
  if (comment.startsWith("/*")) {
    return comment
      .slice(2, comment.endsWith("*/") ? -2 : undefined)
      .split("\n")
      .map((line) => line.replace(/^\s*\*? ?/, ""))
      .join("\n")
      .trim();
  }
  return comment;
}

const renderComment: Renderer = (context, at) => {
  const element = marked(context, at, "div", "comment");
  element.textContent = commentText(node(context, at).value ?? "");
  return element;
};

const renderDocComment: Renderer = (context, at) => {
  const element = marked(context, at, "div", "doc");
  const body = renderText(
    context.document,
    [{ kind: "text", key: `${node(context, at).key}#text`, value: commentText(node(context, at).value ?? ""), raw: false }],
    true
  );
  // The doc comment is one node; its text is not a node of its own.
  for (const run of body.querySelectorAll<HTMLElement>("[data-key], [data-part-of]")) {
    delete run.dataset["key"];
    delete run.dataset["partOf"];
    run.classList.remove("n");
  }
  element.append(...body.childNodes);
  return element;
};

const renderUnparsed: Renderer = (context, at) => {
  const { document } = context;
  const element = marked(context, at, "div", "unparsed");
  element.append(span(document, "unparsed-marker", "Could not be read"));
  const pre = document.createElement("pre");
  pre.textContent = node(context, at).value ?? "";
  element.append(pre);
  return element;
};

/** The renderer of each role; a role added to the protocol fails to compile until it has one. */
const roleRenderers = {
  import: renderImport,
  declaration: renderDeclaration,
  parameter: renderMember,
  stateField: renderMember,
  field: renderMember,
  unionCase: renderUnionCase,
  emit: renderEmit,
  typeReference: renderTypeReference,
  element: renderElement,
  attribute: renderAttribute,
  text: renderTextRun,
  embed: renderEmbed,
  literal: renderLiteral,
  empty: renderEmpty,
  case: renderCase,
  reference: renderReference,
  member: renderMemberAccess,
  operator: renderOperator,
  call: renderCall,
  sequence: renderSequence,
  condition: renderCondition,
  match: renderMatch,
  matchArm: renderMatchArm,
  loop: renderLoop,
  binding: renderBinding,
  comment: renderComment,
  docComment: renderDocComment,
  unparsed: renderUnparsed,
} satisfies Record<SourceRole, Renderer>;

/**
 * The renderers by role. A role without one renders nothing, which the coverage test reports as
 * the keys of the nodes left out; the tests remove one to prove it.
 */
export const renderers: Partial<Record<SourceRole, Renderer>> = { ...roleRenderers };
