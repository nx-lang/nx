/**
 * The Markdown a doc comment uses, rendered as text and elements only: paragraphs, inline and fenced
 * code, emphasis, lists and links. Nothing is parsed as markup, so raw HTML reads as its text, and a
 * link is made only to an `http:` or `https:` address, opening outside the page.
 */

/** A list item's marker, and what follows it. */
const LIST_ITEM = /^\s{0,3}([-*+]|\d{1,9}[.)])\s+(.*)$/;

/** A code fence's opening line: its run of backticks or tildes and an optional language. */
const FENCE = /^\s{0,3}(`{3,}|~{3,})\s*([^`\s]*)\s*$/;

/** The characters a backslash escapes. */
const ESCAPABLE = /[\\`*_[\]()#+\-.!<>{}|~]/;

/** Renders `text`, Markdown, as nodes of `document`. */
export function renderMarkdown(document: Document, text: string): DocumentFragment {
  const fragment = document.createDocumentFragment();
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  let at = 0;
  while (at < lines.length) {
    const line = lines[at]!;
    if (line.trim() === "") {
      at += 1;
      continue;
    }
    const fence = FENCE.exec(line);
    if (fence !== null) {
      const marker = fence[1]!;
      const body: string[] = [];
      at += 1;
      while (at < lines.length && !isClosingFence(lines[at]!, marker)) {
        body.push(lines[at]!);
        at += 1;
      }
      at += 1;
      const pre = document.createElement("pre");
      const code = document.createElement("code");
      code.textContent = body.join("\n");
      pre.append(code);
      fragment.append(pre);
      continue;
    }
    const item = LIST_ITEM.exec(line);
    if (item !== null) {
      const ordered = /\d/.test(item[1]!);
      const list = document.createElement(ordered ? "ol" : "ul");
      if (ordered && parseInt(item[1]!, 10) !== 1) {
        list.setAttribute("start", String(parseInt(item[1]!, 10)));
      }
      while (at < lines.length) {
        const next = LIST_ITEM.exec(lines[at]!);
        if (next === null || /\d/.test(next[1]!) !== ordered) {
          break;
        }
        // An item goes on over the indented lines after it that start no item of their own.
        const words = [next[2]!];
        at += 1;
        while (at < lines.length && /^\s+\S/.test(lines[at]!) && LIST_ITEM.exec(lines[at]!) === null) {
          words.push(lines[at]!.trim());
          at += 1;
        }
        const entry = document.createElement("li");
        renderInline(document, entry, words.join("\n"));
        list.append(entry);
      }
      fragment.append(list);
      continue;
    }
    // A paragraph runs to a blank line, a fence or a list.
    const words: string[] = [];
    while (
      at < lines.length &&
      lines[at]!.trim() !== "" &&
      FENCE.exec(lines[at]!) === null &&
      (words.length === 0 || LIST_ITEM.exec(lines[at]!) === null)
    ) {
      words.push(lines[at]!.trim());
      at += 1;
    }
    const paragraph = document.createElement("p");
    renderInline(document, paragraph, words.join("\n"));
    fragment.append(paragraph);
  }
  return fragment;
}

function isClosingFence(line: string, marker: string): boolean {
  const trimmed = line.trim();
  return trimmed.length >= marker.length && trimmed === marker[0]!.repeat(trimmed.length);
}

/** Renders a run of inline Markdown, `text`, into `parent`. */
function renderInline(document: Document, parent: Node, text: string): void {
  let plain = "";
  const flush = (): void => {
    if (plain !== "") {
      parent.appendChild(document.createTextNode(plain));
      plain = "";
    }
  };
  let at = 0;
  while (at < text.length) {
    const char = text[at]!;
    if (char === "\\" && at + 1 < text.length && ESCAPABLE.test(text[at + 1]!)) {
      plain += text[at + 1];
      at += 2;
      continue;
    }
    if (char === "`") {
      const run = runOf(text, at, "`");
      const close = findRun(text, at + run, "`", run);
      if (close < 0) {
        plain += "`".repeat(run);
        at += run;
        continue;
      }
      flush();
      const code = document.createElement("code");
      const inner = text.slice(at + run, close);
      // One space either side of a span lets it start or end with a backtick.
      code.textContent = /^ .* $/s.test(inner) && inner.trim() !== "" ? inner.slice(1, -1) : inner;
      parent.appendChild(code);
      at = close + run;
      continue;
    }
    if (char === "*" || char === "_") {
      const marker = text[at + 1] === char ? char + char : char;
      const close = opensEmphasis(text, at, marker) ? findEmphasisClose(text, at + marker.length, marker) : -1;
      if (close < 0) {
        plain += marker;
        at += marker.length;
        continue;
      }
      flush();
      const emphasis = document.createElement(marker.length === 2 ? "strong" : "em");
      renderInline(document, emphasis, text.slice(at + marker.length, close));
      parent.appendChild(emphasis);
      at = close + marker.length;
      continue;
    }
    if (char === "[") {
      const link = linkAt(text, at);
      if (link === undefined) {
        plain += char;
        at += 1;
        continue;
      }
      flush();
      const address = webAddress(link.href);
      if (address === undefined) {
        // A link anywhere but the web reads as its words.
        renderInline(document, parent, link.label);
      } else {
        const anchor = document.createElement("a");
        anchor.setAttribute("href", address);
        anchor.setAttribute("target", "_blank");
        anchor.setAttribute("rel", "noopener noreferrer");
        renderInline(document, anchor, link.label);
        parent.appendChild(anchor);
      }
      at = link.end;
      continue;
    }
    plain += char;
    at += 1;
  }
  flush();
}

/** How many of `char` run from `at`. */
function runOf(text: string, at: number, char: string): number {
  let end = at;
  while (text[end] === char) {
    end += 1;
  }
  return end - at;
}

/** Where a run of exactly `length` of `char` starts at or after `from`, or -1. */
function findRun(text: string, from: number, char: string, length: number): number {
  for (let at = from; at < text.length; ) {
    if (text[at] !== char) {
      at += 1;
      continue;
    }
    const run = runOf(text, at, char);
    if (run === length) {
      return at;
    }
    at += run;
  }
  return -1;
}

/** Whether `marker` at `at` can open emphasis: something not a space follows it, and `_` starts a word. */
function opensEmphasis(text: string, at: number, marker: string): boolean {
  const next = text[at + marker.length];
  if (next === undefined || /\s/.test(next)) {
    return false;
  }
  return marker[0] !== "_" || !/[\p{L}\p{N}]/u.test(text[at - 1] ?? "");
}

/** Where emphasis opened by `marker` closes, from `from`, skipping code spans, or -1. */
function findEmphasisClose(text: string, from: number, marker: string): number {
  for (let at = from; at < text.length; at++) {
    const char = text[at]!;
    if (char === "\\") {
      at += 1;
      continue;
    }
    if (char === "`") {
      const run = runOf(text, at, "`");
      const close = findRun(text, at + run, "`", run);
      at = close < 0 ? at + run - 1 : close + run - 1;
      continue;
    }
    if (!text.startsWith(marker, at) || at === from || /\s/.test(text[at - 1]!)) {
      continue;
    }
    // A single marker is not half of a double one.
    if (marker.length === 1 && (text[at + 1] === marker || text[at - 1] === marker)) {
      continue;
    }
    if (marker[0] === "_" && /[\p{L}\p{N}]/u.test(text[at + marker.length] ?? "")) {
      continue;
    }
    return at;
  }
  return -1;
}

/** A link `[label](href)` at `at`: its words, where it points, and where it ends. */
function linkAt(text: string, at: number): { label: string; href: string; end: number } | undefined {
  let depth = 0;
  let close = -1;
  for (let index = at; index < text.length; index++) {
    const char = text[index];
    if (char === "\\") {
      index += 1;
    } else if (char === "[") {
      depth += 1;
    } else if (char === "]") {
      depth -= 1;
      if (depth === 0) {
        close = index;
        break;
      }
    }
  }
  if (close < 0 || text[close + 1] !== "(") {
    return undefined;
  }
  // The address may hold parentheses of its own, in pairs.
  let end = -1;
  for (let index = close + 2, open = 0; index < text.length; index++) {
    if (text[index] === "(") {
      open += 1;
    } else if (text[index] === ")") {
      if (open === 0) {
        end = index;
        break;
      }
      open -= 1;
    }
  }
  if (end < 0) {
    return undefined;
  }
  // The address, without angle brackets or a title after it.
  const href = (text.slice(close + 2, end).trim().split(/\s+/)[0] ?? "").replace(/^<(.*)>$/, "$1");
  return { label: text.slice(at + 1, close), href, end: end + 1 };
}

/** `href` when it is an `http:` or `https:` address, and otherwise nothing. */
function webAddress(href: string): string | undefined {
  try {
    const url = new URL(href);
    return url.protocol === "http:" || url.protocol === "https:" ? url.href : undefined;
  } catch {
    return undefined;
  }
}
