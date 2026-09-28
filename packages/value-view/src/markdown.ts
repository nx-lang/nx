/**
 * The small part of markdown that hovers use: fenced code blocks, highlighted when they are NX,
 * and paragraphs of text with inline code. Anything else is shown as the text it is.
 */
import { colorize, type NxValueHighlighter, type NxValueThemes } from "./highlight.js";
import { appendColored } from "./render.js";

/** Renders `markdown` into a fragment of `document`. */
export function renderMarkdown(
  document: Document,
  markdown: string,
  highlighter: NxValueHighlighter | undefined,
  themes: NxValueThemes
): DocumentFragment {
  const fragment = document.createDocumentFragment();
  for (const block of splitBlocks(markdown)) {
    if (block.kind === "code") {
      const pre = document.createElement("pre");
      pre.className = "hover-code";
      const code = document.createElement("code");
      const colors =
        highlighter !== undefined && block.language === "nx"
          ? colorize(highlighter, block.text, themes)
          : [];
      appendColored(document, code, block.text, 0, block.text.length, colors);
      pre.append(code);
      fragment.append(pre);
    } else {
      const paragraph = document.createElement("p");
      appendInline(document, paragraph, block.text);
      fragment.append(paragraph);
    }
  }
  return fragment;
}

type Block =
  | { readonly kind: "code"; readonly language: string; readonly text: string }
  | { readonly kind: "text"; readonly text: string };

/** Splits markdown into fenced code blocks and paragraphs separated by blank lines. */
export function splitBlocks(markdown: string): Block[] {
  const blocks: Block[] = [];
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  let paragraph: string[] = [];
  const flush = () => {
    const text = paragraph.join("\n").trim();
    if (text !== "") {
      blocks.push({ kind: "text", text });
    }
    paragraph = [];
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]!;
    const fence = /^\s*```(\S*)\s*$/.exec(line);
    if (fence !== null) {
      flush();
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !/^\s*```\s*$/.test(lines[index]!)) {
        code.push(lines[index]!);
        index += 1;
      }
      blocks.push({ kind: "code", language: fence[1] ?? "", text: code.join("\n") });
    } else if (line.trim() === "" || /^\s*(-{3,}|\*{3,})\s*$/.test(line)) {
      flush();
    } else {
      paragraph.push(line);
    }
  }
  flush();
  return blocks;
}

/** Appends text with `inline code` spans. */
function appendInline(document: Document, parent: HTMLElement, text: string): void {
  const parts = text.split(/(`[^`]+`)/);
  for (const part of parts) {
    if (part.length > 2 && part.startsWith("`") && part.endsWith("`")) {
      const code = document.createElement("code");
      code.textContent = part.slice(1, -1);
      parent.append(code);
    } else if (part !== "") {
      parent.append(document.createTextNode(part));
    }
  }
}
