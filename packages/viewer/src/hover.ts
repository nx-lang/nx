/**
 * What a hover says, read from the declaration table alone: a property's type, doc comment and
 * default, and the cases of its union; a reference's target; an element's kind.
 */
import type { SourceDeclaration, SourceNode, SourceProperty } from "@nx-lang/language-protocol";

import { renderText } from "./text.js";
import { declarationOf, type TreeIndex } from "./tree.js";
import { KIND_LABELS } from "./words.js";

function line(document: Document, className: string, ...content: (string | Node)[]): HTMLElement {
  const element = document.createElement("div");
  element.className = className;
  element.append(...content);
  return element;
}

function code(document: Document, text: string): HTMLElement {
  const element = document.createElement("code");
  element.textContent = text;
  return element;
}

/** A doc comment rendered as markdown, carrying no keys. */
function doc(document: Document, markdown: string): HTMLElement {
  const body = renderText(document, [{ kind: "text", key: "", value: markdown, raw: false }], true, "none");
  body.classList.add("hover-doc");
  return body;
}

/** The union a property's type names, if the table describes it. */
function unionOf(index: TreeIndex, property: SourceProperty, value: SourceNode | undefined): SourceDeclaration | undefined {
  const fromCase = value?.role === "case" ? declarationOf(index, value) : undefined;
  if (fromCase?.kind === "union") {
    return fromCase;
  }
  const name = property.type.replace(/[?*+]$/, "");
  return index.tree.declarations.find((entry) => entry.kind === "union" && entry.name === name);
}

/**
 * The hover for the property `name` of the element at `element`: its type, whether it is
 * optional, its default and doc comment, and for a union every case with the current one marked.
 * `value` is the node the element sets the property to, if it sets it.
 */
export function propertyHover(
  document: Document,
  index: TreeIndex,
  element: number,
  name: string,
  value: SourceNode | undefined
): HTMLElement | undefined {
  const elementNode = index.tree.nodes[element];
  const entry = elementNode === undefined ? undefined : declarationOf(index, elementNode);
  const property = entry?.properties?.find((candidate) => candidate.name === name);
  if (property === undefined) {
    return undefined;
  }
  const hover = document.createElement("div");
  const head = line(document, "hover-head", code(document, property.name), " ", code(document, property.type));
  if (property.flags?.includes("optional")) {
    head.append(" ", Object.assign(document.createElement("span"), { className: "tag", textContent: "optional" }));
  }
  hover.append(head);
  if (property.doc !== undefined) {
    hover.append(doc(document, property.doc));
  }
  const union = unionOf(index, property, value);
  if (union?.cases !== undefined) {
    const cases = line(document, "hover-cases");
    for (const unionCase of union.cases) {
      const pill = document.createElement("span");
      const current = value?.role === "case" && value.name === unionCase.name;
      pill.className = current ? "pill current" : "pill";
      pill.textContent = unionCase.name;
      if (current) {
        pill.setAttribute("aria-current", "true");
      }
      cases.append(pill);
    }
    hover.append(cases);
  }
  if (property.default !== undefined) {
    hover.append(line(document, "hover-note", "Default: ", code(document, property.default)));
  }
  return hover;
}

/**
 * The hover for the handler `name` of the element at `element`: the action it answers, found by
 * name in the declaration table as the element's kind declares it (`SearchBox.ValueChanged`, or a
 * top-level `SearchSubmitted`), with its doc comment and the fields it carries.
 */
export function handlerHover(document: Document, index: TreeIndex, element: number, name: string): HTMLElement {
  const elementNode = index.tree.nodes[element];
  const kind = (elementNode === undefined ? undefined : declarationOf(index, elementNode))?.name ?? elementNode?.name ?? "";
  const event = name.replace(/^on/, "");
  const action =
    index.tree.declarations.find((entry) => entry.kind === "action" && entry.name === `${kind}.${event}`) ??
    index.tree.declarations.find((entry) => entry.kind === "action" && entry.name === event);
  const hover = document.createElement("div");
  hover.append(
    line(document, "hover-head", code(document, name), ` · when ${kind} emits `, code(document, action?.name ?? event))
  );
  if (action?.doc !== undefined) {
    hover.append(doc(document, action.doc));
  }
  const fields = action?.properties ?? [];
  if (fields.length > 0) {
    const carries = line(document, "hover-note", "Carries ");
    fields.forEach((field, position) => {
      carries.append(position === 0 ? "" : ", ", code(document, `${field.name}:${field.type}`));
    });
    hover.append(carries);
  }
  return hover;
}

/** The hover for a reference, or for an element's kind: the target's name, kind and doc comment. */
export function declarationHover(document: Document, index: TreeIndex, source: SourceNode): HTMLElement | undefined {
  const entry = declarationOf(index, source);
  if (entry === undefined) {
    return undefined;
  }
  const hover = document.createElement("div");
  const head = line(document, "hover-head", code(document, entry.name), ` · ${KIND_LABELS[entry.kind].toLowerCase()}`);
  if (entry.type !== undefined) {
    head.append(" ", code(document, entry.type));
  }
  hover.append(head);
  if (entry.doc !== undefined) {
    hover.append(doc(document, entry.doc));
  }
  if (entry.module !== index.tree.identity) {
    hover.append(line(document, "hover-note", "From ", code(document, entry.module)));
  }
  return hover;
}
