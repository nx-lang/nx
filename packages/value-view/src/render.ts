/**
 * Building the DOM for a value: one span per node, nested the way the nodes nest, with the text
 * inside colored by the ranges the highlighter produced.
 */
import type { ColoredRange } from "./highlight.js";
import type { NxValueNode, NxValueText } from "./types.js";

/** Roles whose text can span several lines and so can fold. */
const foldableRoles: ReadonlySet<string> = new Set(["record", "property", "sequence"]);

/** What rendering a value produced. */
export interface RenderedValue {
  /** The `<code>` element holding the value. */
  readonly root: HTMLElement;
  /** The element for each node, by node index. */
  readonly elements: readonly HTMLElement[];
}

/** Whether the node's text spans more than one line, so a toggle can fold it. */
export function spansLines(value: NxValueText, node: NxValueNode): boolean {
  return value.text.slice(node.start, node.end).includes("\n");
}

/** Whether only indentation comes before the node on its line, so the gutter beside it is free. */
export function startsLine(value: NxValueText, node: NxValueNode): boolean {
  const lineStart = value.text.lastIndexOf("\n", node.start - 1) + 1;
  return /^[ \t]*$/.test(value.text.slice(lineStart, node.start));
}

/**
 * Renders `value` into a new `<code>` element.
 *
 * <para>A record, property or sequence that spans several lines gets a fold toggle in the gutter
 * when it starts its line. One that starts partway along a line has no gutter beside it, and a
 * toggle there would sit on the text; the node that does start the line folds it along with its
 * own. The value as a whole does not fold, since that would hide everything there is to read. The
 * formatter puts at most one foldable node at the start of a line, so a line has at most one
 * toggle.</para>
 */
export function renderValue(
  document: Document,
  value: NxValueText,
  colors: readonly ColoredRange[]
): RenderedValue {
  const children: number[][] = value.nodes.map(() => []);
  const roots: number[] = [];
  value.nodes.forEach((node, index) => {
    if (node.parent === undefined || children[node.parent] === undefined) {
      roots.push(index);
    } else {
      children[node.parent]!.push(index);
    }
  });

  const foldable = value.nodes.map(
    (node) =>
      node.parent !== undefined &&
      foldableRoles.has(node.role) &&
      spansLines(value, node) &&
      startsLine(value, node)
  );
  const elements: HTMLElement[] = new Array(value.nodes.length);

  const renderNode = (index: number): HTMLElement => {
    const node = value.nodes[index]!;
    const element = document.createElement("span");
    element.className = "node";
    element.dataset["node"] = String(index);
    element.dataset["role"] = node.role;
    element.tabIndex = -1;
    if (node.declaration !== undefined) {
      element.classList.add("declared");
    }
    elements[index] = element;

    const childIndices = children[index]!;
    if (foldable[index] === true) {
      const toggle = document.createElement("button");
      toggle.className = "fold";
      toggle.type = "button";
      toggle.tabIndex = -1;
      toggle.setAttribute("aria-expanded", "true");
      toggle.dataset["label"] = describeForLabel(node);
      toggle.setAttribute("aria-label", `Fold ${toggle.dataset["label"]}`);
      element.append(toggle);

      const full = document.createElement("span");
      full.className = "full";
      renderRange(full, node.start, node.end, childIndices);
      element.append(full);

      const folded = document.createElement("span");
      folded.className = "folded";
      folded.hidden = true;
      const lineEnd = value.text.indexOf("\n", node.start);
      appendColored(document, folded, value.text, node.start, lineEnd, colors);
      folded.append(document.createTextNode(" "));
      const ellipsis = document.createElement("button");
      ellipsis.className = "ellipsis";
      ellipsis.type = "button";
      ellipsis.tabIndex = -1;
      ellipsis.textContent = "…";
      ellipsis.setAttribute("aria-label", `Unfold ${describeForLabel(node)}`);
      folded.append(ellipsis);
      element.append(folded);
      element.classList.add("foldable");
    } else {
      renderRange(element, node.start, node.end, childIndices);
    }
    return element;
  };

  const renderRange = (
    parent: HTMLElement,
    start: number,
    end: number,
    childIndices: readonly number[]
  ) => {
    let cursor = start;
    for (const child of childIndices) {
      const node = value.nodes[child]!;
      appendColored(document, parent, value.text, cursor, node.start, colors);
      parent.append(renderNode(child));
      cursor = node.end;
    }
    appendColored(document, parent, value.text, cursor, end, colors);
  };

  const root = document.createElement("code");
  renderRange(root, 0, value.text.length, roots);
  return { root, elements };
}

/**
 * Appends `text.slice(start, end)` to `parent`, as colored spans where `colors` covers it and plain
 * text where it does not.
 */
export function appendColored(
  document: Document,
  parent: HTMLElement,
  text: string,
  start: number,
  end: number,
  colors: readonly ColoredRange[]
): void {
  if (end <= start) {
    return;
  }
  let cursor = start;
  for (let index = firstRangeEndingAfter(colors, start); index < colors.length; index += 1) {
    const range = colors[index]!;
    if (range.start >= end) {
      break;
    }
    const from = Math.max(range.start, start);
    const to = Math.min(range.end, end);
    if (from > cursor) {
      parent.append(document.createTextNode(text.slice(cursor, from)));
    }
    if (to > from) {
      const span = document.createElement("span");
      span.className = "t";
      if (range.light !== undefined) {
        span.style.setProperty("--l", range.light);
      }
      if (range.dark !== undefined) {
        span.style.setProperty("--d", range.dark);
      }
      if (range.fontStyle !== undefined) {
        if (range.fontStyle & 1) {
          span.classList.add("i");
        }
        if (range.fontStyle & 2) {
          span.classList.add("b");
        }
        if (range.fontStyle & 4) {
          span.classList.add("u");
        }
      }
      span.textContent = text.slice(from, to);
      parent.append(span);
    }
    cursor = Math.max(cursor, to);
  }
  if (cursor < end) {
    parent.append(document.createTextNode(text.slice(cursor, end)));
  }
}

function firstRangeEndingAfter(colors: readonly ColoredRange[], offset: number): number {
  let low = 0;
  let high = colors.length;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if (colors[middle]!.end <= offset) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low;
}

function describeForLabel(node: NxValueNode): string {
  if (node.role === "property" && node.name !== undefined) {
    return node.name;
  }
  return node.type ?? node.role;
}
