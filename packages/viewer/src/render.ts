/**
 * The reading view: one renderer for each role of the source tree. Each turns a node and its
 * children into DOM and marks the result with the node's key, so the reading is lossless by
 * construction: a node no renderer draws is missing from the rendering, which the coverage test
 * catches. Only the element, attribute, literal and case renderers consult the declaration table,
 * for property order, defaults and the viewers it carries.
 */
import type {
  SourceNode,
  SourceProperty,
  SourcePropertyViewer,
  SourceRole,
  SourceValueFormat,
  SourceViewer,
} from "@nx-lang/language-protocol";

import { renderText, type KeyMode, type TextPiece } from "./text.js";
import { childrenIn, declarationOf, indexedNodeAtSpan, nodeOf, type TreeIndex } from "./tree.js";
import {
  BINARY_READINGS,
  eventWords,
  KIND_LABELS,
  POSTFIX_READINGS,
  PREFIX_READINGS,
} from "./words.js";

/** What every renderer is given. */
export interface RenderContext {
  readonly document: Document;
  readonly index: TreeIndex;
  /**
   * Whether every omitted default shows as a ghost row. A default whose property the kind's viewer
   * gives a label, a phrase or a place shows either way, since the viewer says the property matters
   * to a reader.
   */
  readonly ghosts: boolean;
  /**
   * Whether this is a copy of nodes rendered elsewhere, as a reference's expansion is: a copy
   * carries each node's key as `data-ref-key`, so the original stays the one node with the key.
   */
  readonly copy: boolean;
  /** Whether a value formatted as `image` loads its picture; true unless the host turns it off. */
  readonly images?: boolean;
}

/** Turns the node at `at` into DOM, or nothing. */
export type Renderer = (context: RenderContext, at: number, options: RenderOptions) => HTMLElement;

/** What a parent tells the renderer of a child. */
export interface RenderOptions {
  /** The name of the declaration whose value this element is, shown on its card. */
  readonly declarationName?: string;
  /** Whether a string literal reads as words of a sentence, with no value style. */
  readonly inSentence?: boolean;
  /** How a string literal shows, from the viewer of the property it is bound to. */
  readonly format?: SourceValueFormat;
  /** The viewer of the boolean property a literal is bound to, whose phrases it may read as. */
  readonly phrases?: SourcePropertyViewer;
  /**
   * Whether a boolean or a case reads as the word it is written with, as a member's default and a
   * viewer declaration's values do.
   */
  readonly asWritten?: boolean;
  /** The doc comments written over a declaration, which read inside its card. */
  readonly docs?: readonly number[];
}

/** Renders the node at `at` by its role; nothing when no renderer draws the role. */
export function renderNode(context: RenderContext, at: number, options: RenderOptions = {}): HTMLElement | undefined {
  const renderer = renderers[nodeOf(context.index, at).role];
  return renderer?.(context, at, options);
}

/**
 * Renders the whole document: its top-level nodes in order, with its imports in one strip, each
 * run of viewer declarations in one strip, and a declaration's doc comments inside its card.
 */
export function renderDocument(context: RenderContext): HTMLElement {
  const { document, index } = context;
  const root = document.createElement("div");
  root.className = "document";
  let imports: HTMLElement | undefined;
  let viewers: HTMLElement | undefined;
  let viewerNames: string[] = [];
  let docs: number[] = [];
  const flushDocs = () => {
    for (const doc of docs) {
      appendNode(context, root, doc);
    }
    docs = [];
  };
  index.tree.nodes.forEach((node, at) => {
    if (node.parent !== undefined) {
      return;
    }
    if (node.role === "docComment") {
      docs.push(at);
      return;
    }
    if (node.role === "import") {
      flushDocs();
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
    if (node.role !== "declaration") {
      flushDocs();
      viewers = undefined;
      appendNode(context, root, at);
      return;
    }
    const options: RenderOptions = docs.length > 0 ? { docs } : {};
    docs = [];
    if (!has(node, "viewer")) {
      viewers = undefined;
      appendNode(context, root, at, options);
      return;
    }
    // Viewers say how types read, which already shows on every element: a run of them folds into
    // one strip that names them.
    if (viewers === undefined) {
      viewers = detailsStrip(document, "Viewers");
      viewers.classList.add("viewer-strip");
      viewerNames = [];
      root.append(viewers);
    }
    appendNode(context, viewers.querySelector(".details-body")!, at, options);
    viewerNames.push(node.name ?? "");
    viewers
      .querySelector("summary")!
      .replaceChildren(`Viewers · ${viewerNames.length} `, span(document, "strip-names", viewerNames.join(", ")));
  });
  flushDocs();
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

/** The element node that holds the node at `at`, through any condition in its tag. */
function elementHolding(context: RenderContext, at: number): number | undefined {
  for (let parent = node(context, at).parent; parent !== undefined; parent = node(context, parent).parent) {
    if (node(context, parent).role === "element") {
      return parent;
    }
  }
  return undefined;
}

/** The viewer of the property an attribute sets, from its element's kind. */
function propertyViewerOf(context: RenderContext, attribute: number): SourcePropertyViewer | undefined {
  const element = elementHolding(context, attribute);
  const viewer = element === undefined ? undefined : declarationOf(context.index, node(context, element))?.viewer;
  const name = node(context, attribute).name;
  return viewer?.properties?.find((property) => property.name === name);
}

/** The phrase a boolean property reads as for `value`, when its viewer sets one. */
function phraseFor(viewer: SourcePropertyViewer | undefined, value: string | undefined): string | undefined {
  if (value === "true") {
    return viewer?.whenTrue;
  }
  if (value === "false") {
    return viewer?.whenFalse;
  }
  return undefined;
}

function node(context: RenderContext, at: number): SourceNode {
  return nodeOf(context.index, at);
}

function has(node: SourceNode, flag: NonNullable<SourceNode["flags"]>[number]): boolean {
  return node.flags?.includes(flag) ?? false;
}

/** How text rendered in `context` carries its keys. */
function keysOf(context: RenderContext): KeyMode {
  return context.copy ? "copy" : "original";
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
  if (rendered === undefined) {
    return;
  }
  const source = node(context, at);
  if (!isLineComment(source) || !standsAlone(context, source)) {
    parent.append(rendered);
    return;
  }
  // A comment written over several lines is a run of line comments that each stand alone on their
  // line: they read as one note.
  const previous = parent.lastElementChild;
  const previousLine = previous === null ? undefined : noteEndLines.get(previous);
  if (previous !== null && previousLine !== undefined && previousLine + 1 === source.range.start.line) {
    let notes = previous;
    if (!previous.classList.contains("notes")) {
      notes = context.document.createElement("div");
      notes.className = "notes";
      previous.replaceWith(notes);
      notes.append(previous);
    }
    notes.append(rendered);
    noteEndLines.set(notes, source.range.end.line);
    return;
  }
  parent.append(rendered);
  noteEndLines.set(rendered, source.range.end.line);
}

/** The last source line of each rendered line comment, or run of them, that a next line comment may join. */
const noteEndLines = new WeakMap<Element, number>();

/** Whether only whitespace comes before the node on its line, so it does not trail other code. */
function standsAlone(context: RenderContext, source: SourceNode): boolean {
  const { bytes } = context.index;
  for (let at = source.range.startByte - 1; at >= 0 && bytes[at] !== 0x0a; at--) {
    if (bytes[at] !== 0x20 && bytes[at] !== 0x09 && bytes[at] !== 0x0d) {
      return false;
    }
  }
  return true;
}

function isLineComment(source: SourceNode): boolean {
  const text = source.value ?? "";
  return source.role === "comment" && text.startsWith("//") && !text.startsWith("///");
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

const renderDeclaration: Renderer = (context, at, options) => {
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
  // `viewer Choice = <ElementViewer … />` reads a line to an element, headed "viewer Choice".
  if (has(source, "viewer")) {
    return renderViewerDeclaration(context, at, options);
  }
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
    // The declaration's doc comments read under the card's heading.
    const heading = section.querySelector(":scope > .card > .card-header");
    const docs = document.createElement("div");
    docs.className = "declaration-docs";
    appendAll(context, docs, options.docs ?? []);
    if (docs.childElementCount > 0) {
      if (heading === null) {
        section.prepend(docs);
      } else {
        heading.after(docs);
      }
    }
    return section;
  }

  const section = marked(context, at, "section", "declaration");
  const header = document.createElement("div");
  header.className = "declaration-header";
  // Spaces between the parts, so the header reads as words to a screen reader and when copied.
  for (const flag of source.flags ?? []) {
    header.append(span(document, "tag", flag), " ");
  }
  if (entry !== undefined) {
    header.append(span(document, "declaration-kind", KIND_LABELS[entry.kind]), " ");
  }
  header.append(span(document, "declaration-name", source.name ?? ""));

  // What fits beside the name reads there, as it does in the source: `type Swatch = string` reads
  // "Type Swatch = string", and a function's return type reads after an arrow.
  const types = children.filter((child) => index.slots[child] === "type");
  const headed = new Set<number>();
  if (types.length > 0 && types.every((child) => node(context, child).role === "typeReference")) {
    header.append(" ", span(document, "declaration-sign", typeSign(entry?.kind)));
    for (const child of types) {
      header.append(" ");
      appendNode(context, header, child);
      headed.add(child);
    }
  }
  // `type ThemeMode = light | dark` reads on one line too, when no case carries fields or a
  // comment.
  const caseLine =
    members.length > 0 &&
    children.every((child) => node(context, child).role === "unionCase" && (index.children[child] ?? []).length === 0)
      ? document.createElement("span")
      : undefined;
  if (caseLine !== undefined) {
    caseLine.className = "case-line";
    header.append(" ", span(document, "declaration-sign", "="), " ", caseLine);
  }
  if (!context.copy) {
    header.append(sourceButton(document, source.key));
  }
  section.append(header);
  appendAll(context, section, options.docs ?? []);

  const groupOf = (child: number): string | undefined => {
    const childNode = node(context, child);
    const slot = index.slots[child];
    if (MEMBER_ROLES.has(childNode.role)) {
      return memberGroup(childNode.role, entry?.kind);
    }
    if (slot === "type") {
      return typeGroup(entry?.kind);
    }
    if (!isNote(childNode)) {
      return slot === "body" ? (entry?.kind === "component" ? "Shows" : "Returns") : "Is";
    }
    return undefined;
  };
  const inBody = children.filter((child) => !headed.has(child));
  // A heading tells one group of a declaration from the next, so a lone group needs none.
  const groups = new Set(inBody.map(groupOf).filter((name) => name !== undefined));
  const body = document.createElement("div");
  body.className = "declaration-body";
  let group: string | undefined;
  for (const child of inBody) {
    if (caseLine !== undefined) {
      if (caseLine.childElementCount > 0) {
        caseLine.append(span(document, "case-bar", " | "));
      }
      appendNode(context, caseLine, child);
      continue;
    }
    // A comment belongs with what it is written over, so it reads under that group's heading.
    const grouped = isNote(node(context, child))
      ? inBody.slice(inBody.indexOf(child)).find((later) => !isNote(node(context, later)))
      : child;
    const groupName = grouped === undefined ? undefined : groupOf(grouped);
    if (groupName !== undefined && groupName !== group) {
      if (groups.size > 1) {
        body.append(label(document, groupName));
      }
      group = groupName;
    }
    appendNode(context, body, child);
  }
  if (body.childElementCount > 0) {
    section.append(body);
  }
  return section;
};

/** The sign between a declaration's name and its type in its header. */
function typeSign(kind: string | undefined): string {
  switch (kind) {
    case "alias":
      return "=";
    case "function":
      return "→";
    default:
      return ":";
  }
}

/**
 * A viewer declaration: its name, then a line to each of its elements, as the source has them. The
 * document folds each run of these into one strip.
 */
function renderViewerDeclaration(context: RenderContext, at: number, options: RenderOptions): HTMLElement {
  const { document, index } = context;
  const source = node(context, at);
  const block = marked(context, at, "section", "viewer-block");
  const header = document.createElement("div");
  header.className = "viewer-header";
  for (const flag of source.flags ?? []) {
    if (flag !== "viewer") {
      header.append(span(document, "tag", flag), " ");
    }
  }
  header.append(span(document, "declaration-kind", "viewer"), " ", span(document, "declaration-name", source.name ?? ""));
  block.append(header);
  appendAll(context, block, options.docs ?? []);
  for (const child of index.children[at] ?? []) {
    if (node(context, child).role === "element") {
      block.append(renderViewerElement(context, child, true));
    } else {
      appendNode(context, block, child);
    }
  }
  return block;
}

/**
 * An element of a viewer declaration on one line: its name, then each attribute as its name and
 * value, with the elements it holds indented under it. An element that says what it is `of` leads
 * with that in place of its name, so `<PropertyViewer of=id label="Key" />` reads "id label Key".
 */
function renderViewerElement(context: RenderContext, at: number, top: boolean): HTMLElement {
  const { document, index } = context;
  const source = node(context, at);
  const block = marked(context, at, "div", "viewer-element");
  block.setAttribute("role", "group");
  block.setAttribute("aria-label", source.name ?? "");
  const line = document.createElement("div");
  line.className = "viewer-line";
  const nested = document.createElement("div");
  nested.className = "viewer-nested";
  const children = index.children[at] ?? [];
  const subject = children.find((child) => node(context, child).role === "attribute" && node(context, child).name === "of");
  if (subject === undefined) {
    line.append(span(document, "viewer-kind", source.name ?? ""));
  } else {
    block.title = source.name ?? "";
    line.append(viewerAttribute(context, subject, false));
  }
  for (const child of children) {
    const childNode = node(context, child);
    if (child === subject) {
      continue;
    }
    if (childNode.role === "element") {
      nested.append(renderViewerElement(context, child, false));
    } else if (childNode.role !== "attribute") {
      appendNode(context, nested, child);
    } else if (has(childNode, "content")) {
      // The elements an element holds are the value of its content property.
      const held = marked(context, child, "div", "viewer-held");
      for (const item of index.children[child] ?? []) {
        if (node(context, item).role === "element") {
          held.append(renderViewerElement(context, item, false));
        } else {
          appendNode(context, held, item);
        }
      }
      nested.append(held);
    } else {
      line.append(viewerAttribute(context, child, true));
    }
  }
  if (top && !context.copy) {
    line.append(sourceButton(document, source.key));
  }
  block.append(line);
  if (nested.childElementCount > 0) {
    block.append(nested);
  }
  return block;
}

/** An attribute on a viewer element's line: its name, when `named`, and its value. */
function viewerAttribute(context: RenderContext, at: number, named: boolean): HTMLElement {
  const { document, index } = context;
  const source = node(context, at);
  const element = marked(context, at, "span", named ? "viewer-attribute" : "viewer-attribute subject");
  if (named) {
    const name = span(document, "label", source.name ?? "");
    name.dataset["property"] = source.name ?? "";
    name.tabIndex = 0;
    element.append(name, " ");
  }
  // A viewer's own values read as written: the case a `CaseViewer` is `of` is its name, not the
  // label the viewer gives it.
  appendAll(context, element, index.children[at] ?? [], { asWritten: true });
  return element;
}

/** The heading over a declaration's type: what an alias is, what a function returns, a value's type. */
function typeGroup(kind: string | undefined): string {
  switch (kind) {
    case "alias":
      return "Is";
    case "function":
      return "Returns";
    default:
      return "Type";
  }
}

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
    const isDefault = index.slots[child] === "default";
    if (isDefault) {
      row.append(" ", span(document, "member-default-label", "="));
    }
    row.append(" ");
    // A default reads as it is written: `required:boolean = true` reads "required boolean = true".
    appendNode(context, row, child, isDefault ? { asWritten: true } : {});
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

/** The parts of a card a viewer can bind a property to, in the order the header shows them. */
const PARTS = ["title", "subtitle", "badge", "image"] as const;

type Part = (typeof PARTS)[number];

const renderElement: Renderer = (context, at, options) => {
  const { document, index } = context;
  const source = node(context, at);
  if (has(source, "stateUpdate")) {
    return renderStateUpdate(context, at);
  }
  const entry = declarationOf(index, source);
  const viewer = entry?.viewer;
  // A layout this version does not know reads as a card, which folds nothing away.
  const layout = viewer?.layout === "row" || viewer?.layout === "inline" ? viewer.layout : "card";
  const card = marked(context, at, layout === "inline" ? "span" : "div", layout === "card" ? "card" : `card ${layout}-layout`);
  card.tabIndex = 0;
  card.setAttribute("role", "group");
  // A kind reads by its name as written, so the reading and the source use one word for it.
  const kindName = viewer?.name ?? source.name ?? "";
  card.setAttribute("aria-label", entry === undefined ? source.name ?? "" : kindName);

  const header = document.createElement(layout === "inline" ? "span" : "div");
  header.className = "card-header";
  const kind = span(document, entry === undefined ? "kind unresolved" : "kind", kindName);
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
  card.append(header);

  const children = index.children[at] ?? [];
  const grouped = units(context, children);
  const attributeUnits = grouped.filter((unit) => node(context, lastOf(unit)).role === "attribute");
  const otherUnits = grouped.filter((unit) => node(context, lastOf(unit)).role !== "attribute");
  const byName = new Map(attributeUnits.map((unit) => [node(context, lastOf(unit)).name ?? "", unit]));
  const placed = new Set<number[]>();

  const rows = document.createElement("div");
  rows.className = "rows";
  // A row or inline layout folds everything but its heading; any layout folds what it places there.
  const details = detailsStrip(document, "Details");
  const detailRows = document.createElement("div");
  detailRows.className = "rows";
  details.querySelector(".details-body")!.append(detailRows);
  const folded = layout !== "card";

  // The attributes bound to the card's parts move into its header, keeping their keys. An inline
  // element's line holds only its title; its other parts read as rows in its details.
  const partNames = new Set<string>();
  for (const part of layout === "inline" ? (["title"] as const) : PARTS) {
    const name = viewer?.[part];
    const unit = name === undefined ? undefined : byName.get(name);
    if (name === undefined || unit === undefined || placed.has(unit)) {
      continue;
    }
    partNames.add(name);
    placed.add(unit);
    appendAll(context, folded ? detailRows : rows, unit.slice(0, -1));
    header.append(renderPart(context, lastOf(unit), part));
  }
  if (!context.copy) {
    header.append(sourceButton(document, source.key));
  }

  const declared: readonly SourceProperty[] = entry?.properties ?? [];
  const order = viewer?.properties?.map((property) => property.name) ?? declared.map((property) => property.name);
  const set = attributesSetBy(context, at);
  for (const name of order) {
    if (partNames.has(name)) {
      continue;
    }
    const propertyViewer = viewer?.properties?.find((candidate) => candidate.name === name);
    const container = folded || propertyViewer?.place === "details" ? detailRows : rows;
    const unit = byName.get(name);
    const property = declared.find((candidate) => candidate.name === name);
    if (unit !== undefined && !placed.has(unit)) {
      appendAll(context, container, unit);
      placed.add(unit);
    } else if ((context.ghosts || saysHowToRead(propertyViewer)) && property?.default !== undefined && !set.has(name)) {
      container.append(ghostRow(document, property, propertyViewer));
    }
  }
  for (const unit of attributeUnits) {
    if (!placed.has(unit)) {
      appendAll(context, folded ? detailRows : rows, unit);
    }
  }
  if (rows.childElementCount > 0) {
    card.append(rows);
  }

  // What the element holds: text, embeds, nested elements and logic, in source order.
  const content = document.createElement("div");
  content.className = "content";
  appendContent(context, content, otherUnits.flat(), source.textType === "markdown");
  if (content.childElementCount > 0) {
    if (folded) {
      details.querySelector(".details-body")!.append(content);
    } else {
      card.append(content);
    }
  }
  if (detailRows.childElementCount === 0) {
    detailRows.remove();
  }
  if (details.querySelector(".details-body")!.childElementCount > 0) {
    card.append(details);
  }
  return card;
};

/** An attribute bound to a part of its card: its value alone, marked with the attribute's key. */
function renderPart(context: RenderContext, at: number, part: Part): HTMLElement {
  const { document, index } = context;
  const source = node(context, at);
  const element = marked(context, at, "span", `part ${part}`);
  // The part's property, hovered like a row's label.
  element.dataset["property"] = source.name ?? "";
  element.tabIndex = 0;
  const format: SourceValueFormat | undefined = part === "image" ? "image" : undefined;
  for (const child of index.children[at] ?? []) {
    appendNode(context, element, child, format === undefined ? {} : { format });
  }
  return element;
}

/**
 * Appends an element's content: runs of text and embeds as one body of prose, markdown when the
 * element's text type is, and everything else as it renders, in source order.
 */
function appendContent(context: RenderContext, container: HTMLElement, children: readonly number[], markdown: boolean): void {
  const { document } = context;
  let pieces: TextPiece[] = [];
  const flushText = () => {
    if (pieces.length > 0) {
      container.append(renderText(document, pieces, markdown, keysOf(context)));
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

/**
 * Whether a viewer says how its property reads or where it goes, which marks the property as one a
 * reader should see even at its default. A format does not count: a property takes one from the
 * viewer of its type's alias without being named itself.
 */
function saysHowToRead(viewer: SourcePropertyViewer | undefined): boolean {
  return (
    viewer !== undefined &&
    (viewer.label !== undefined || viewer.whenTrue !== undefined || viewer.whenFalse !== undefined || viewer.place === "details")
  );
}

function ghostRow(document: Document, property: SourceProperty, viewer: SourcePropertyViewer | undefined): HTMLElement {
  const row = document.createElement("div");
  row.className = "row ghost";
  const phrase = phraseFor(viewer, property.default);
  if (phrase !== undefined) {
    // The default reads as its phrase, as a set value would.
    row.classList.add("phrase-row");
    const value = document.createElement("span");
    value.className = "row-value";
    const words = span(document, "ghost-value phrase", phrase);
    words.dataset["property"] = property.name;
    words.tabIndex = 0;
    // Italic alone marks the phrase as a default.
    value.append(words);
    row.append(value);
    return row;
  }
  const name = span(document, "label", viewer?.label ?? property.name);
  name.dataset["property"] = property.name;
  name.tabIndex = 0;
  const value = document.createElement("span");
  value.className = "row-value";
  value.append(span(document, "ghost-value", property.default ?? ""), span(document, "tag", "default"));
  row.append(name, value);
  return row;
}

/** `<State.Update a={…} b={…} />` in a component: one "set a to …" line per attribute. */
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
    const when = span(document, "when", `When ${eventWords(source.name ?? "")}`);
    // The handler's property, hovered like any attribute's name.
    when.dataset["property"] = source.name ?? "";
    when.tabIndex = 0;
    row.append(when);
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

  const viewer = propertyViewerOf(context, at);
  const values = children.filter((child) => !isNote(node(context, child)));
  const only = values.length === 1 ? node(context, values[0]!) : undefined;
  if (only?.role === "literal" && phraseFor(viewer, only.value) !== undefined) {
    // A boolean with a phrase for its value reads as the phrase, in place of its row.
    row.classList.add("phrase-row");
    const value = document.createElement("div");
    value.className = "row-value";
    appendAll(context, value, children, { phrases: viewer! });
    row.append(value);
    return row;
  }

  const name = span(document, "label", viewer?.label ?? source.name ?? "");
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
    appendAll(context, value, children, viewer?.format === undefined ? {} : { format: viewer.format });
  }
  row.append(name, value);
  return row;
};

const renderTextRun: Renderer = (context, at) => {
  const source = node(context, at);
  return renderText(
    context.document,
    [{ kind: "text", key: source.key, value: source.value ?? "", raw: has(source, "raw") }],
    false,
    keysOf(context)
  );
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
    if (options.format !== undefined && options.inSentence !== true) {
      return formatted(context, at, unquote(text), options.format);
    }
    const element = marked(context, at, "span", options.inSentence === true ? "words" : "value string");
    element.textContent = unquote(text);
    return element;
  }
  const phrase = phraseFor(options.phrases, text);
  if (phrase !== undefined) {
    const element = marked(context, at, "span", "value phrase");
    element.textContent = phrase;
    // The phrase stands for its property's row, so it is hovered like the row's label.
    element.dataset["property"] = node(context, source.parent ?? at).name ?? "";
    element.tabIndex = 0;
    return element;
  }
  if ((text === "true" || text === "false") && options.asWritten === true) {
    const element = marked(context, at, "span", `value boolean written ${text}`);
    element.textContent = text;
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

/** Whether `url` is an `http:` or `https:` URL, the only kind the viewer links to or loads. */
function isWebUrl(url: string): boolean {
  return /^https?:\/\/[^\s]+$/i.test(url.trim());
}

function link(document: Document, href: string, text: string): HTMLAnchorElement {
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.textContent = text;
  anchor.target = "_blank";
  anchor.rel = "noopener noreferrer";
  return anchor;
}

/**
 * A string literal shown by its property's format, still without its quotes and carrying its key.
 * A format changes how the value looks, never what it is, and nothing but an `http:` or `https:`
 * URL is linked to or loaded.
 */
function formatted(context: RenderContext, at: number, value: string, format: SourceValueFormat): HTMLElement {
  const { document } = context;
  const element = marked(context, at, "span", `value formatted ${format}`);
  switch (format) {
    case "url":
      element.append(isWebUrl(value) ? link(document, value.trim(), value) : value);
      break;
    case "email":
      element.append(/^[^\s@:/]+@[^\s@/]+$/.test(value) ? link(document, `mailto:${value}`, value) : value);
      break;
    case "phone": {
      const digits = value.replace(/[^\d+]/g, "");
      element.append(digits === "" ? value : link(document, `tel:${digits}`, value));
      break;
    }
    case "image":
      if (isWebUrl(value) && context.images !== false) {
        const picture = document.createElement("img");
        picture.src = value.trim();
        picture.alt = "";
        picture.loading = "lazy";
        picture.referrerPolicy = "no-referrer";
        picture.className = "picture";
        element.append(picture, span(document, "picture-url", value));
      } else {
        element.append(isWebUrl(value) ? link(document, value.trim(), value) : value);
      }
      break;
    case "color": {
      const swatch = span(document, "swatch", "");
      // A color, never anything else: `background-color` takes no URL.
      swatch.style.backgroundColor = value;
      swatch.setAttribute("aria-hidden", "true");
      element.append(swatch, value);
      break;
    }
    case "markdown":
      element.append(renderText(document, [{ kind: "text", key: "", value, raw: false }], true, "none"));
      break;
    // `text`, and a format this version does not know, show the value as it is.
    default:
      element.textContent = value;
      break;
  }
  return element;
}

const renderEmpty: Renderer = (context, at) => {
  const element = marked(context, at, "span", "value empty");
  element.textContent = "empty";
  return element;
};

const renderCase: Renderer = (context, at, options) => {
  const { document, index } = context;
  const source = node(context, at);
  const element = marked(context, at, "span", "pill case");
  const qualifier = index.children[at] ?? [];
  if (qualifier.length > 0) {
    const muted = span(document, "qualifier", "");
    appendAll(context, muted, qualifier);
    element.append(muted, " ");
  }
  const viewer: SourceViewer | undefined = declarationOf(index, source)?.viewer;
  const label = viewer?.cases?.find((candidate) => candidate.name === source.name)?.label;
  if (label === undefined || options.asWritten === true) {
    element.append(source.name ?? "");
  } else {
    // The case's label, with its name and example in the hover.
    element.classList.add("labelled");
    element.tabIndex = 0;
    element.append(label);
  }
  return element;
};

const renderSequence: Renderer = (context, at) => {
  const { index } = context;
  const children = index.children[at] ?? [];
  const blocks = children.some((child) => isBlockRole(node(context, child).role));
  if (blocks) {
    const element = marked(context, at, "div", "sequence blocks");
    appendAll(context, element, children);
    return element;
  }
  // Items in a line read as a list: "a, b, c".
  const element = marked(context, at, "span", "sequence");
  appendJoined(context, element, children, ", ");
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
  const value = entry?.valueRange === undefined ? undefined : indexedNodeAtSpan(index, entry.valueRange.startByte, entry.valueRange.endByte);
  if (value !== undefined && !context.copy) {
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
  // The doc comment is one node; its text is not a node of its own.
  const body = renderText(
    context.document,
    [{ kind: "text", key: "", value: commentText(node(context, at).value ?? ""), raw: false }],
    true,
    "none"
  );
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
