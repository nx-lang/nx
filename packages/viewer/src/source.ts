/**
 * The source reading: the document's own text, character for character, with each node of the
 * source tree wrapped in a span that carries its key. Nothing is rewritten or moved, so it reads as
 * the editor shows it, and selection and the hand-off to an editor work as they do in the card
 * reading. Colors come from the host, as ranges over the text, so a host colors the reading with
 * the highlighter and themes it colors its editor with.
 */
import type { SourceNode } from "@nx-lang/language-protocol";

import type { TreeIndex } from "./tree.js";

/** One colored range of the text, in UTF-16 offsets. Ranges are in text order and do not overlap. */
export interface NxViewerColor {
  readonly start: number;
  readonly end: number;
  /** The color under a light color scheme, as CSS writes one. */
  readonly light?: string;
  /** The color under a dark color scheme. */
  readonly dark?: string;
  /** Font style bits, as Shiki counts them: 1 italic, 2 bold, 4 underline. */
  readonly fontStyle?: number;
}

/** The UTF-16 offset of each UTF-8 byte offset of `text` that starts a character, and of its end. */
function utf16Offsets(text: string, byteLength: number): Uint32Array {
  const offsets = new Uint32Array(byteLength + 1);
  let byte = 0;
  for (let at = 0; at < text.length; ) {
    const code = text.codePointAt(at)!;
    const bytes = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
    const units = code > 0xffff ? 2 : 1;
    for (let part = 0; part < bytes && byte + part <= byteLength; part++) {
      offsets[byte + part] = at;
    }
    byte += bytes;
    at += units;
  }
  offsets[byteLength] = text.length;
  return offsets;
}

/** Renders the document's text with its nodes marked, colored by `colors`. */
export function renderSource(
  document: Document,
  index: TreeIndex,
  colors: readonly NxViewerColor[]
): HTMLElement {
  const { text, tree } = index;
  const offsets = utf16Offsets(text, index.bytes.length);
  const startOf = (node: SourceNode): number => offsets[Math.min(node.range.startByte, index.bytes.length)]!;
  const endOf = (node: SourceNode): number => offsets[Math.min(node.range.endByte, index.bytes.length)]!;

  const root = document.createElement("pre");
  root.className = "source";

  const marks = spacingMarks(index, startOf);

  // A piece of the text in one color, or in none: a text node, or a span when it has a color or a
  // spacing class to carry.
  const piece = (parent: HTMLElement, from: number, to: number, range: NxViewerColor | undefined): void => {
    // The piece is cut where its spacing changes, so each run takes one class.
    let at = from;
    while (at < to) {
      const mark = marks.get(at);
      let end = at + 1;
      while (end < to && marks.get(end) === mark) {
        end += 1;
      }
      if (range === undefined && mark === undefined) {
        parent.append(text.slice(at, end));
        at = end;
        continue;
      }
      const element = document.createElement("span");
      element.className = [range === undefined ? "" : "t", mark ?? ""].filter((name) => name !== "").join(" ");
      if (range?.light !== undefined) {
        element.style.setProperty("--l", range.light);
      }
      if (range?.dark !== undefined) {
        element.style.setProperty("--d", range.dark);
      }
      const style = range?.fontStyle ?? 0;
      if ((style & 1) !== 0) {
        element.style.fontStyle = "italic";
      }
      if ((style & 2) !== 0) {
        element.style.fontWeight = "bold";
      }
      if ((style & 4) !== 0) {
        element.style.textDecoration = "underline";
      }
      element.textContent = text.slice(at, end);
      parent.append(element);
      at = end;
    }
  };

  // The colors are in text order and the text is written in order, so one cursor walks them.
  let color = 0;
  const writeText = (parent: HTMLElement, from: number, to: number): void => {
    let at = from;
    while (at < to) {
      while (color < colors.length && colors[color]!.end <= at) {
        color += 1;
      }
      const range = colors[color];
      if (range === undefined || range.start >= to) {
        piece(parent, at, to, undefined);
        return;
      }
      if (range.start > at) {
        piece(parent, at, range.start, undefined);
        at = range.start;
      }
      const end = Math.min(range.end, to);
      piece(parent, at, end, range);
      at = end;
    }
  };

  const uiText = uiTextNodes(index);

  // A string a person will read: its words in the prose font and the reading's own color, between
  // quotes left quiet. The words sit in a box that begins after the opening quote, so a line the
  // string breaks, or one too long for the pane, continues under the first and not at the margin.
  const writeUIText = (element: HTMLElement, start: number, end: number): void => {
    element.classList.add("uitext");
    const quote = (): HTMLElement => {
      const mark = document.createElement("span");
      mark.className = "uitext-quote";
      mark.textContent = '"';
      return mark;
    };
    const block = document.createElement("span");
    block.className = "uitext-block";
    // How far into its line the words begin, which is what the box may not be wider than the pane by.
    const lineStart = text.lastIndexOf("\n", start) + 1;
    block.style.setProperty("--column", String(start + 1 - lineStart));
    const body = document.createElement("span");
    body.className = "uitext-body";
    body.append(text.slice(start + 1, end - 1), quote());
    block.append(body);
    element.append(quote(), block);
  };

  // Writes the nodes at `children` and the text between them, from `from` to `to`. A node whose
  // range does not sit inside what is left to write is left unmarked rather than written twice.
  const writeNodes = (parent: HTMLElement, children: readonly number[], from: number, to: number): void => {
    let at = from;
    const ordered = [...children].sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
    for (const child of ordered) {
      const node = tree.nodes[child]!;
      const start = startOf(node);
      const end = endOf(node);
      if (start < at || end > to || end < start) {
        continue;
      }
      writeText(parent, at, start);
      const element = document.createElement("span");
      element.className = "n";
      element.dataset["role"] = node.role;
      element.dataset["key"] = node.key;
      // A comment is prose, which a host may set in another font than the code around it.
      if (node.role === "comment" || node.role === "docComment") {
        element.classList.add("prose");
      }
      if (uiText.has(child) && node.role === "literal" && end - start >= 2 && text[start] === '"' && text[end - 1] === '"') {
        writeUIText(element, start, end);
      } else {
        if (uiText.has(child)) {
          element.classList.add("uitext", "uitext-body");
        }
        writeNodes(element, index.children[child] ?? [], start, end);
      }
      parent.append(element);
      at = end;
    }
    writeText(parent, at, to);
  };

  const top = tree.nodes.flatMap((node, at) => (node.parent === undefined ? [at] : []));
  writeNodes(root, top, 0, text.length);
  markIndentation(document, root);
  return root;
}

/** How a character of the text is spaced in the reading, where that differs from how it is written. */
type SpacingMark = "attribute-equals" | "attribute-space";

/**
 * The spacing the reading gives an attribute's `=`: a little room on each side, whether the source
 * writes `name=value` or `name = value`. The `=` is marked to take that room, and a single space
 * beside it is marked to give its own up, so both spellings read alike. An `=` set off by more than
 * one space is left as its author spaced it. Keyed by UTF-16 offset.
 */
function spacingMarks(index: TreeIndex, startOf: (node: SourceNode) => number): Map<number, SpacingMark> {
  const { text, tree } = index;
  const marks = new Map<number, SpacingMark>();
  tree.nodes.forEach((node, at) => {
    if (node.role !== "attribute" || node.name === undefined) {
      return;
    }
    const values = index.children[at] ?? [];
    if (values.length === 0) {
      return;
    }
    const start = startOf(node);
    const value = Math.min(...values.map((child) => startOf(tree.nodes[child]!)));
    // Between the attribute's name and its value: spaces, `=`, spaces, and at most the `{` or `<`
    // that opens the value.
    const equals = text.indexOf("=", start + node.name.length);
    if (equals < 0 || equals >= value || text.slice(start + node.name.length, equals).trim() !== "") {
      return;
    }
    let before = equals;
    while (before > start && text[before - 1] === " ") {
      before -= 1;
    }
    let after = equals + 1;
    while (after < text.length && text[after] === " ") {
      after += 1;
    }
    if (equals - before > 1 || after - (equals + 1) > 1) {
      return;
    }
    marks.set(equals, "attribute-equals");
    for (let space = before; space < equals; space++) {
      marks.set(space, "attribute-space");
    }
    for (let space = equals + 1; space < after; space++) {
      marks.set(space, "attribute-space");
    }
  });
  return marks;
}

/** The name the type of text shown to people goes by. An alias for now; it may become built in. */
const UI_TEXT = "uitext";

/**
 * The nodes that are text shown to people: the string literals and element text that a property,
 * a field, a parameter or a declaration typed `uitext` holds, where `uitext` is the type itself
 * or any alias that leads to it. Which strings these are comes from the declarations, never from
 * how a string looks.
 */
function uiTextNodes(index: TreeIndex): Set<number> {
  const { tree } = index;
  const declarations = tree.declarations;
  const known = new Map<string, boolean>();
  const isUIText = (spelled: string | undefined, depth = 0): boolean => {
    const name = (spelled ?? "").replace(/[?+*]$/, "").trim();
    if (name === UI_TEXT) {
      return true;
    }
    if (name === "" || depth > 8) {
      return false;
    }
    const cached = known.get(name);
    if (cached !== undefined) {
      return cached;
    }
    const alias = declarations.find((entry) => entry.kind === "alias" && entry.name === name);
    const result = alias !== undefined && isUIText(alias.type, depth + 1);
    known.set(name, result);
    return result;
  };

  const found = new Set<number>();
  // The strings and text under `at`, through any expression, but not inside an element it holds:
  // that element's properties have types of their own.
  const mark = (at: number): void => {
    for (const child of index.children[at] ?? []) {
      const node = tree.nodes[child]!;
      if (node.role === "text" || (node.role === "literal" && (node.value ?? "").startsWith('"'))) {
        found.add(child);
      } else if (node.role !== "element" && node.role !== "typeReference") {
        mark(child);
      }
    }
  };

  tree.nodes.forEach((node, at) => {
    if (node.role === "attribute") {
      const element = node.parent === undefined ? undefined : tree.nodes[node.parent];
      const declared = element?.declaration === undefined ? undefined : declarations[element.declaration];
      const property = declared?.properties?.find((candidate) => candidate.name === node.name);
      if (isUIText(property?.type)) {
        mark(at);
      }
      return;
    }
    if (node.role === "field" || node.role === "parameter" || node.role === "stateField" || node.role === "declaration") {
      const typed = (index.children[at] ?? []).some((child) => {
        const candidate = tree.nodes[child]!;
        return candidate.role === "typeReference" && index.slots[child] !== "extends" && isUIText(candidate.value);
      });
      if (typed) {
        mark(at);
      }
    }
  });
  return found;
}

/**
 * Wraps each line's leading whitespace in a span, so indentation keeps a fixed width whatever font
 * the text around it is set in. Nothing is added or removed: the spans hold the spaces themselves.
 */
function markIndentation(document: Document, root: HTMLElement): void {
  const texts: Text[] = [];
  const collect = (node: Node): void => {
    for (const child of Array.from(node.childNodes)) {
      if (child.nodeType === 3) {
        texts.push(child as Text);
      } else {
        collect(child);
      }
    }
  };
  collect(root);
  let lineStart = true;
  for (const node of texts) {
    const value = node.data;
    const fragment = document.createDocumentFragment();
    let at = 0;
    let changed = false;
    while (at < value.length) {
      if (lineStart) {
        let end = at;
        while (end < value.length && (value[end] === " " || value[end] === "\t")) {
          end += 1;
        }
        if (end > at) {
          const indent = document.createElement("span");
          indent.className = "indent";
          indent.textContent = value.slice(at, end);
          fragment.append(indent);
          changed = true;
          at = end;
        }
        if (at >= value.length) {
          // The line's indentation may go on in the next piece of text.
          break;
        }
        lineStart = false;
      }
      const newline = value.indexOf("\n", at);
      const end = newline < 0 ? value.length : newline + 1;
      fragment.append(value.slice(at, end));
      at = end;
      lineStart = newline >= 0;
    }
    if (changed) {
      node.replaceWith(fragment);
    }
  }
}
