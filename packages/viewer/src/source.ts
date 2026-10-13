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
        parent.append(text.slice(at, to));
        return;
      }
      if (range.start > at) {
        parent.append(text.slice(at, range.start));
        at = range.start;
      }
      const end = Math.min(range.end, to);
      const piece = document.createElement("span");
      piece.className = "t";
      if (range.light !== undefined) {
        piece.style.setProperty("--l", range.light);
      }
      if (range.dark !== undefined) {
        piece.style.setProperty("--d", range.dark);
      }
      const style = range.fontStyle ?? 0;
      if ((style & 1) !== 0) {
        piece.style.fontStyle = "italic";
      }
      if ((style & 2) !== 0) {
        piece.style.fontWeight = "bold";
      }
      if ((style & 4) !== 0) {
        piece.style.textDecoration = "underline";
      }
      piece.textContent = text.slice(at, end);
      parent.append(piece);
      at = end;
    }
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
      writeNodes(element, index.children[child] ?? [], start, end);
      parent.append(element);
      at = end;
    }
    writeText(parent, at, to);
  };

  const top = tree.nodes.flatMap((node, at) => (node.parent === undefined ? [at] : []));
  writeNodes(root, top, 0, text.length);
  return root;
}
