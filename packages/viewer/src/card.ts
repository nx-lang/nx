/**
 * What a hover card says about a node of the source tree, from the tree alone, in the shapes the
 * editor's hover uses: a signature in NX, then the doc comment, as Markdown.
 */
import type { SourceDeclaration, SourceNode, SourceProperty } from "@nx-lang/language-protocol";

import { attributeElement, elementProperties, type TreeIndex } from "./tree.js";

/** A hover card's content. */
export interface Card {
  /** What the node is, in NX: `component Shape extends Node`, `(property) Type: ShapeType = Rectangle`. */
  readonly signature: string;
  /** The doc comment, as Markdown, when there is one. */
  readonly doc?: string;
}

/** The roles of the values a card describes, where the reading shows them. */
export const VALUE_ROLES: ReadonlySet<string> = new Set(["case", "reference", "call", "member"]);

/**
 * The card for the node at `at`: an element's or a declaration's declaration, an attribute's
 * property, a union case, or a reference, a member or a call. Nothing for any other node, nor for
 * one the tree gives neither a type nor a doc.
 */
export function cardOf(index: TreeIndex, at: number): Card | undefined {
  const { tree } = index;
  const node = tree.nodes[at];
  if (node === undefined) {
    return undefined;
  }
  switch (node.role) {
    case "element":
      return elementCard(index, node);
    case "declaration": {
      if (node.declaration === undefined) {
        return undefined;
      }
      // A function's return type, where it is written.
      const written = (index.children[at] ?? []).find(
        (child) => index.slots[child] === "type" && tree.nodes[child]!.role === "typeReference"
      );
      const returns = written === undefined ? undefined : tree.nodes[written]!.value;
      return declarationCard(index, node.declaration, isElementFunction(index, at), returns);
    }
    case "attribute":
      return propertyCard(index, at);
    case "case":
      return caseCard(index, node.declaration, node.name ?? "", node.type);
    case "reference":
    case "member":
      return referenceCard(index, node);
    case "call": {
      // A call is what it calls: the function's signature and doc.
      const callee = (index.children[at] ?? []).find((child) => index.slots[child] === "callee");
      return callee === undefined ? undefined : cardOf(index, callee);
    }
    default:
      return undefined;
  }
}

/** Whether the declaration node at `at` is an element function, written `let <Name …>`. */
export function isElementFunction(index: TreeIndex, at: number): boolean {
  const node = index.tree.nodes[at]!;
  const written = new TextDecoder().decode(index.bytes.subarray(node.range.startByte, node.range.startByte + 48));
  return /^(?:(?:export|private)\s+)*let\s*</.test(written);
}

/** An element's card: its declaration's, or its case's for an element of a union case. */
function elementCard(index: TreeIndex, node: SourceNode): Card | undefined {
  const declaration = node.declaration === undefined ? undefined : index.tree.declarations[node.declaration];
  if (declaration === undefined) {
    return undefined;
  }
  const name = node.name ?? "";
  if (declaration.kind === "union" && name.includes(".")) {
    return caseCard(index, node.declaration, name.slice(name.lastIndexOf(".") + 1), undefined);
  }
  // An element names a function only when the function is an element function.
  return declarationCard(index, node.declaration!, declaration.kind === "function");
}

/** A declaration's signature, as the editor's hover begins it, and its doc. */
function declarationCard(index: TreeIndex, at: number, elementFunction: boolean, returns?: string): Card | undefined {
  const declaration = index.tree.declarations[at];
  if (declaration === undefined) {
    return undefined;
  }
  return withDoc(declarationSignature(index, at, declaration, elementFunction, returns), declaration.doc);
}

function declarationSignature(
  index: TreeIndex,
  at: number,
  declaration: SourceDeclaration,
  elementFunction: boolean,
  returns?: string
): string {
  const { tree } = index;
  const base = declaration.bases?.[0] === undefined ? undefined : tree.declarations[declaration.bases[0]];
  const extending = base === undefined ? "" : ` extends ${base.name}`;
  const name = declaration.name;
  switch (declaration.kind) {
    case "component": {
      // The modifiers are known where the queried document declares it.
      const flags = tree.nodes.find((node) => node.role === "declaration" && node.declaration === at)?.flags ?? [];
      const modifiers = `${flags.includes("abstract") ? "abstract " : ""}${flags.includes("external") ? "external " : ""}`;
      return `${modifiers}component ${name}${extending}`;
    }
    case "record":
    case "union":
      return `type ${name}${extending}`;
    case "alias":
      return declaration.type === undefined ? `type ${name}` : `type ${name} = ${declaration.type}`;
    case "action":
      return `action ${name}${extending}`;
    case "value":
      return declaration.type === undefined ? `let ${name}` : `let ${name}: ${declaration.type}`;
    case "function": {
      if (elementFunction) {
        return `let <${name} />`;
      }
      const params = (declaration.properties ?? [])
        .map((property) => `${property.name}${optional(property.flags)}:${property.type}`)
        .join(", ");
      return `let ${name}(${params})${returns === undefined ? "" : `: ${returns}`}`;
    }
  }
}

/**
 * `(property) name?: Type = default` and its doc, from the declaration of the element it is on, under
 * a property-list condition too.
 */
function propertyCard(index: TreeIndex, at: number): Card | undefined {
  const { tree } = index;
  const node = tree.nodes[at]!;
  const element = attributeElement(tree, at);
  const property =
    element === undefined ? undefined : elementProperties(tree, tree.nodes[element]!)?.find((candidate) => candidate.name === node.name);
  return property === undefined ? undefined : propertyCardOf(property);
}

/** A property's card, `(property) name?: Type = default` and its doc, as for one an element does not write. */
export function propertyCardOf(property: SourceProperty): Card {
  const fallback = property.default === undefined ? "" : ` = ${property.default}`;
  return withDoc(`(property) ${property.name}${optional(property.flags)}: ${property.type}${fallback}`, property.doc);
}

/** `(case) Union.case` and the case's doc. */
function caseCard(index: TreeIndex, union: number | undefined, name: string, type: string | undefined): Card | undefined {
  const declaration = union === undefined ? undefined : index.tree.declarations[union];
  if (declaration?.kind === "union") {
    const known = declaration.cases?.find((candidate) => candidate.name === name);
    return withDoc(`(case) ${declaration.name}.${name}`, known?.doc);
  }
  return type === undefined ? undefined : { signature: `(case) ${type}` };
}

/**
 * A reference's or a member's card: a function it names as the function's signature, and anything
 * else as its name and type, with the doc of what it refers to.
 */
function referenceCard(index: TreeIndex, node: SourceNode): Card | undefined {
  const at = node.declaration;
  const declaration = at === undefined ? undefined : index.tree.declarations[at];
  if (declaration?.kind === "function" && node.role === "reference") {
    // A function's type reads `<function n:int />: int`; the signature keeps what it returns.
    const returns = /\/>\s*:\s*(.+)$/.exec(node.type ?? "")?.[1];
    return withDoc(declarationSignature(index, at!, declaration, false, returns), declaration.doc);
  }
  if (node.type === undefined && declaration === undefined) {
    return undefined;
  }
  if (node.type === undefined) {
    return declarationCard(index, at!, false);
  }
  return withDoc(`${node.name ?? ""}: ${node.type}`, declaration?.doc);
}

function optional(flags: readonly string[] | undefined): string {
  return flags?.includes("optional") === true ? "?" : "";
}

function withDoc(signature: string, doc: string | undefined): Card {
  return doc === undefined || doc.trim() === "" ? { signature } : { signature, doc: doc.trim() };
}
