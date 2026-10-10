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

/** A styled stretch of inline text: its whole extent, the part between its markers, and its tag. */
interface InlineSpan {
  readonly start: number;
  readonly end: number;
  readonly innerStart: number;
  readonly innerEnd: number;
  readonly tag: "strong" | "em" | "code";
}

/** A run of `*` or `_` while emphasis is matched: where it is, how much is left, and its flanking. */
interface Delimiter {
  readonly char: string;
  readonly length: number;
  /** The run's unused characters are `start` to `end`; openers use theirs from the end. */
  start: number;
  end: number;
  readonly canOpen: boolean;
  readonly canClose: boolean;
}

/**
 * Inline markdown between `start` and `end`: code spans, then `*` and `_` emphasis and strong
 * emphasis matched by CommonMark's delimiter algorithm, so nested emphasis (`*a **b** c*`,
 * `***x***`) and literal asterisks and underscores (`2 * 3`, `snake_case`) read as CommonMark
 * renders them. Backslash escapes and links are not markdown a body uses, and stay as written.
 */
function renderInline(
  document: Document,
  parent: HTMLElement,
  body: Body,
  start: number,
  end: number,
  emit: Emit
): void {
  const spans = inlineSpans(body, start, end);
  // Matched spans nest, so the outermost first, and each renders what it holds.
  spans.sort((a, b) => a.start - b.start || b.end - a.end);
  const render = (into: HTMLElement, from: number, to: number) => {
    let at = from;
    for (const span of spans) {
      if (span.start < at || span.end > to) {
        continue;
      }
      emit(into, at, span.start);
      const styled = document.createElement(span.tag);
      if (span.tag === "code") {
        emit(styled, span.innerStart, span.innerEnd);
      } else {
        render(styled, span.innerStart, span.innerEnd);
      }
      into.append(styled);
      at = span.end;
    }
    emit(into, at, to);
  };
  render(parent, start, end);
}

/** The code spans and emphasis between `start` and `end`. */
function inlineSpans(body: Body, start: number, end: number): InlineSpan[] {
  const spans: InlineSpan[] = [];
  const delimiters: Delimiter[] = [];
  let at = start;
  while (at < end) {
    const char = body.chars[at]!;
    if (char !== "`" && char !== "*" && char !== "_") {
      at += 1;
      continue;
    }
    let after = at;
    while (after < end && body.chars[after] === char) {
      after += 1;
    }
    if (char === "`") {
      // A code span closes at the next run of exactly as many backticks; with none, they are text.
      const close = closingBackticks(body, after, end, after - at);
      if (close >= 0) {
        // One space on each side is padding, so a span can start or end with a backtick, as long
        // as the content is not all spaces.
        const padded =
          body.chars[after] === " " &&
          body.chars[close - 1] === " " &&
          body.chars.slice(after, close).some((inner) => inner !== " " && inner !== "");
        const [innerStart, innerEnd] = padded ? [after + 1, close - 1] : [after, close];
        spans.push({ start: at, end: close + (after - at), innerStart, innerEnd, tag: "code" });
        at = close + (after - at);
      } else {
        at = after;
      }
      continue;
    }
    delimiters.push({ char, length: after - at, start: at, end: after, ...flanking(body, at, after, start, end, char) });
    at = after;
  }
  matchEmphasis(delimiters, spans);
  return spans;
}

function closingBackticks(body: Body, from: number, end: number, length: number): number {
  let at = from;
  while (at < end) {
    if (body.chars[at] !== "`") {
      at += 1;
      continue;
    }
    let after = at;
    while (after < end && body.chars[after] === "`") {
      after += 1;
    }
    if (after - at === length) {
      return at;
    }
    at = after;
  }
  return -1;
}

/**
 * Whether the run from `first` to `after` can open and close emphasis, by CommonMark's flanking
 * rules: a run opens when followed by neither whitespace nor, unless after whitespace or
 * punctuation, punctuation, and closes symmetrically; a run of `_` also does not open or close
 * inside a word.
 */
function flanking(
  body: Body,
  first: number,
  after: number,
  start: number,
  end: number,
  char: string
): { canOpen: boolean; canClose: boolean } {
  const before = classOf(body, first - 1, start, end);
  const next = classOf(body, after, start, end);
  const left = next !== "space" && (next !== "punctuation" || before !== "word");
  const right = before !== "space" && (before !== "punctuation" || next !== "word");
  if (char === "*") {
    return { canOpen: left, canClose: right };
  }
  return {
    canOpen: left && (!right || before === "punctuation"),
    canClose: right && (!left || next === "punctuation"),
  };
}

/** What the character at `at` is for flanking: whitespace (the edge of the text counts), punctuation, or part of a word. */
function classOf(body: Body, at: number, start: number, end: number): "space" | "punctuation" | "word" {
  if (at < start || at >= end) {
    return "space";
  }
  const char = body.chars[at]!;
  // A folded space is blanked to an empty string, and an embed stands as a private-use character.
  if (char === "" || /\s/u.test(char)) {
    return "space";
  }
  return /[\p{P}\p{S}]/u.test(char) ? "punctuation" : "word";
}

/**
 * Pairs openers and closers as CommonMark's "process emphasis" does: each closer, left to right,
 * takes the nearest opener of the same character that the rule of three allows, two characters
 * from each for strong emphasis when both have two, else one; delimiters between them can no longer
 * match. What no closer takes stays text.
 */
function matchEmphasis(delimiters: Delimiter[], spans: InlineSpan[]): void {
  const stack = delimiters.slice();
  let closerIndex = 0;
  while (closerIndex < stack.length) {
    const closer = stack[closerIndex]!;
    if (!closer.canClose) {
      closerIndex += 1;
      continue;
    }
    let openerIndex = closerIndex - 1;
    while (openerIndex >= 0) {
      const opener = stack[openerIndex]!;
      const ruleOfThree =
        (opener.canClose || closer.canOpen) &&
        (opener.length + closer.length) % 3 === 0 &&
        !(opener.length % 3 === 0 && closer.length % 3 === 0);
      if (opener.char === closer.char && opener.canOpen && !ruleOfThree) {
        break;
      }
      openerIndex -= 1;
    }
    if (openerIndex < 0) {
      if (!closer.canOpen) {
        stack.splice(closerIndex, 1);
      } else {
        closerIndex += 1;
      }
      continue;
    }
    const opener = stack[openerIndex]!;
    const used = opener.end - opener.start >= 2 && closer.end - closer.start >= 2 ? 2 : 1;
    spans.push({
      start: opener.end - used,
      end: closer.start + used,
      innerStart: opener.end,
      innerEnd: closer.start,
      tag: used === 2 ? "strong" : "em",
    });
    opener.end -= used;
    closer.start += used;
    // Delimiters between the two are inside the span and can no longer match across it.
    stack.splice(openerIndex + 1, closerIndex - openerIndex - 1);
    closerIndex = openerIndex + 1;
    if (opener.end === opener.start) {
      stack.splice(openerIndex, 1);
      closerIndex -= 1;
    }
    if (closer.end === closer.start) {
      stack.splice(closerIndex, 1);
    }
  }
}
