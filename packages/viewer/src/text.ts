/**
 * Element text as prose: plain runs with their indentation folded away, or a `markdown` body
 * rendered as the small part of markdown a body uses. A body is a sequence of text runs and
 * `@{…}` embeds; each run's key goes on the first piece of the rendering it produced, and later
 * pieces of the same run point back at it, so a run split over two paragraphs is still one node.
 */

/** One piece of an element's text: a run of text, or an embed already rendered. */
export type TextPiece =
  | { readonly kind: "text"; readonly key: string; readonly value: string; readonly raw: boolean }
  | { readonly kind: "embed"; readonly element: HTMLElement };

/** Where each character of a body came from: a text run, by position, or an embed. */
interface Body {
  readonly chars: string[];
  /** For each character, the index of its piece. */
  readonly owners: number[];
}

/**
 * What a rendered run says about its node: `original` carries the key, as the one rendering of
 * the node; `copy` carries it as `data-ref-key`, as a reference's expansion does; `none` carries
 * nothing, for text that is not a node, such as a doc comment's or a hover's.
 */
export type KeyMode = "original" | "copy" | "none";

/** The private-use character an embed stands as while the body is parsed. */
const EMBED = "";

/** Renders `pieces` as prose, or as markdown when `markdown` is set, keyed as `keys` says. */
export function renderText(
  document: Document,
  pieces: readonly TextPiece[],
  markdown: boolean,
  keys: KeyMode
): HTMLElement {
  const container = document.createElement("div");
  container.className = markdown ? "text-body markdown" : "text-body";
  const emitted = new Set<number>();
  const body = dedent(gather(pieces));

  const emit = (parent: HTMLElement, start: number, end: number) => {
    let at = start;
    while (at < end) {
      const owner = body.owners[at]!;
      let stop = at + 1;
      while (stop < end && body.owners[stop] === owner) {
        stop += 1;
      }
      const piece = pieces[owner]!;
      if (piece.kind === "embed") {
        if (!emitted.has(owner)) {
          parent.append(piece.element);
          emitted.add(owner);
        }
      } else {
        parent.append(runSpan(document, piece, body.chars.slice(at, stop).join(""), !emitted.has(owner), keys));
        emitted.add(owner);
      }
      at = stop;
    }
  };

  if (markdown) {
    renderMarkdown(document, container, body, emit);
  } else if (pieces.some((piece) => piece.kind === "text" && piece.raw)) {
    const pre = document.createElement("pre");
    pre.className = "raw";
    emit(pre, 0, body.chars.length);
    container.append(pre);
  } else {
    const paragraph = document.createElement("p");
    emit(paragraph, ...trimmed(body, 0, body.chars.length, true));
    container.append(paragraph);
  }

  // A run that produced nothing visible, whitespace or markdown syntax alone, still carries its key.
  pieces.forEach((piece, position) => {
    if (!emitted.has(position)) {
      if (piece.kind === "embed") {
        container.append(piece.element);
      } else {
        container.append(runSpan(document, piece, "", true, keys));
      }
    }
  });
  return container;
}

function runSpan(
  document: Document,
  piece: Extract<TextPiece, { kind: "text" }>,
  text: string,
  first: boolean,
  keys: KeyMode
): HTMLElement {
  const span = document.createElement("span");
  span.textContent = text;
  if (keys === "none") {
    span.className = "text";
    return span;
  }
  span.className = "n text";
  span.dataset["role"] = "text";
  if (!first) {
    span.dataset["partOf"] = piece.key;
  } else if (keys === "copy") {
    span.dataset["refKey"] = piece.key;
  } else {
    span.dataset["key"] = piece.key;
  }
  return span;
}

function gather(pieces: readonly TextPiece[]): Body {
  const chars: string[] = [];
  const owners: number[] = [];
  pieces.forEach((piece, position) => {
    const text = piece.kind === "embed" ? EMBED : piece.value;
    for (const char of text) {
      chars.push(char);
      owners.push(position);
    }
  });
  return { chars, owners };
}

/**
 * Removes the indentation every non-blank line shares, as a body indented under its tag is meant
 * to be read, and the blank lines that open and close it.
 */
function dedent(body: Body): Body {
  const lines = splitLines(body);
  const indents = lines
    .filter(([start, end]) => body.chars.slice(start, end).some((char) => char.trim() !== ""))
    .map(([start, end]) => {
      let at = start;
      while (at < end && (body.chars[at] === " " || body.chars[at] === "\t")) {
        at += 1;
      }
      return at - start;
    });
  const indent = indents.length === 0 ? 0 : Math.min(...indents);
  const chars: string[] = [];
  const owners: number[] = [];
  for (const [start, end, newline] of lines) {
    for (let at = Math.min(start + indent, end); at < end; at += 1) {
      chars.push(body.chars[at]!);
      owners.push(body.owners[at]!);
    }
    if (newline !== undefined) {
      chars.push("\n");
      owners.push(body.owners[newline]!);
    }
  }
  // Leading and trailing blank lines are layout, not text.
  let first = 0;
  while (first < chars.length && /\s/.test(chars[first]!)) {
    first += 1;
  }
  let last = chars.length;
  while (last > first && /\s/.test(chars[last - 1]!)) {
    last -= 1;
  }
  return { chars: chars.slice(first, last), owners: owners.slice(first, last) };
}

/** Each line's start and end, and the position of the newline ending it, if any. */
function splitLines(body: Body): [number, number, number | undefined][] {
  const lines: [number, number, number | undefined][] = [];
  let start = 0;
  body.chars.forEach((char, at) => {
    if (char === "\n") {
      lines.push([start, at, at]);
      start = at + 1;
    }
  });
  lines.push([start, body.chars.length, undefined]);
  return lines;
}

/** The range between `start` and `end` without surrounding whitespace, folding runs of it. */
function trimmed(body: Body, start: number, end: number, fold: boolean): [number, number] {
  if (fold) {
    for (let at = start; at < end; at += 1) {
      if (/\s/.test(body.chars[at]!)) {
        body.chars[at] = " ";
      }
    }
    // Collapse a run of spaces to one by blanking all but the first.
    for (let at = end - 1; at > start; at -= 1) {
      if (body.chars[at] === " " && body.chars[at - 1] === " ") {
        body.chars[at] = "";
      }
    }
  }
  let from = start;
  let to = end;
  while (from < to && /^\s*$/.test(body.chars[from]!)) {
    from += 1;
  }
  while (to > from && /^\s*$/.test(body.chars[to - 1]!)) {
    to -= 1;
  }
  return [from, to];
}

type Emit = (parent: HTMLElement, start: number, end: number) => void;

/**
 * The block structure of a markdown body: headings, bulleted and numbered lists, and paragraphs,
 * separated by blank lines. Inline, `**strong**`, `__strong__`, `*emphasis*`, `_emphasis_` and
 * `` `code` ``.
 */
function renderMarkdown(document: Document, container: HTMLElement, body: Body, emit: Emit): void {
  const lines = splitLines(body);
  let list: HTMLElement | undefined;
  let paragraph: [number, number] | undefined;

  const flushParagraph = () => {
    if (paragraph !== undefined) {
      const p = document.createElement("p");
      renderInline(document, p, body, ...trimmed(body, paragraph[0], paragraph[1], true), emit);
      container.append(p);
      paragraph = undefined;
    }
  };

  for (const [start, end] of lines) {
    const line = body.chars.slice(start, end).join("");
    const heading = /^(#{1,6})\s+/.exec(line);
    const bullet = /^\s*[-*+]\s+/.exec(line);
    const numbered = /^\s*\d+[.)]\s+/.exec(line);
    if (line.trim() === "") {
      flushParagraph();
      list = undefined;
    } else if (heading !== null) {
      flushParagraph();
      list = undefined;
      const h = document.createElement(`h${Math.min(heading[1]!.length + 2, 6)}`);
      renderInline(document, h, body, ...trimmed(body, start + heading[0].length, end, true), emit);
      container.append(h);
    } else if (bullet !== null || numbered !== null) {
      flushParagraph();
      const tag = bullet !== null ? "ul" : "ol";
      if (list === undefined || list.tagName.toLowerCase() !== tag) {
        list = document.createElement(tag);
        container.append(list);
      }
      const item = document.createElement("li");
      const marker = (bullet ?? numbered)![0].length;
      renderInline(document, item, body, ...trimmed(body, start + marker, end, true), emit);
      list.append(item);
    } else {
      list = undefined;
      paragraph = paragraph === undefined ? [start, end] : [paragraph[0], end];
    }
  }
  flushParagraph();
}

const INLINE_MARKERS: readonly { readonly open: string; readonly tag: string }[] = [
  { open: "**", tag: "strong" },
  { open: "__", tag: "strong" },
  { open: "`", tag: "code" },
  { open: "*", tag: "em" },
  { open: "_", tag: "em" },
];

function renderInline(
  document: Document,
  parent: HTMLElement,
  body: Body,
  start: number,
  end: number,
  emit: Emit
): void {
  let plain = start;
  let at = start;
  while (at < end) {
    const marker = INLINE_MARKERS.find(({ open }) => matches(body, at, end, open) && canOpen(body, at, end, open));
    const close = marker === undefined ? -1 : findClose(body, at + marker.open.length, end, marker.open);
    if (marker === undefined || close < 0) {
      // A run of `*` or `_` that cannot open is skipped whole, so its later characters do not open
      // either: `snake__case__x` keeps its underscores.
      const char = body.chars[at];
      at += 1;
      if (char === "*" || char === "_") {
        while (at < end && body.chars[at] === char) {
          at += 1;
        }
      }
      continue;
    }
    emit(parent, plain, at);
    const styled = document.createElement(marker.tag);
    if (marker.tag === "code") {
      emit(styled, at + marker.open.length, close);
    } else {
      renderInline(document, styled, body, at + marker.open.length, close, emit);
    }
    parent.append(styled);
    at = close + marker.open.length;
    plain = at;
  }
  emit(parent, plain, end);
}

function matches(body: Body, at: number, end: number, text: string): boolean {
  if (at + text.length > end) {
    return false;
  }
  for (let offset = 0; offset < text.length; offset += 1) {
    if (body.chars[at + offset] !== text[offset]) {
      return false;
    }
  }
  return true;
}

/** Whether the character at `at` separates words: whitespace, the body's edge, or a folded space. */
function isSpace(body: Body, at: number, start: number, end: number): boolean {
  if (at < start || at >= end) {
    return true;
  }
  const char = body.chars[at]!;
  return char === "" || /\s/.test(char);
}

function isWordCharacter(body: Body, at: number, start: number, end: number): boolean {
  return at >= start && at < end && /[\p{L}\p{N}]/u.test(body.chars[at]!);
}

/**
 * Whether `marker` at `at` can open emphasis, by CommonMark's flanking rule in its common cases:
 * an emphasis marker is followed by a non-space, and `_` does not open inside a word, so
 * `find_plans_for_team` and `2 * 3 * 4` keep their characters. Code spans open anywhere.
 */
function canOpen(body: Body, at: number, end: number, marker: string): boolean {
  if (marker === "`") {
    return true;
  }
  if (isSpace(body, at + marker.length, 0, end)) {
    return false;
  }
  return !marker.startsWith("_") || !isWordCharacter(body, at - 1, 0, end);
}

/** Whether `marker` at `at` can close emphasis: preceded by a non-space, and `_` not inside a word. */
function canClose(body: Body, at: number, end: number, marker: string): boolean {
  if (marker === "`") {
    return true;
  }
  if (isSpace(body, at - 1, 0, end)) {
    return false;
  }
  return !marker.startsWith("_") || !isWordCharacter(body, at + marker.length, 0, end);
}

/** Where `marker` closes after `from`, with something between; -1 when it does not. */
function findClose(body: Body, from: number, end: number, marker: string): number {
  for (let at = from + 1; at + marker.length <= end; at += 1) {
    if (matches(body, at, end, marker) && canClose(body, at, end, marker)) {
      return at;
    }
  }
  return -1;
}
