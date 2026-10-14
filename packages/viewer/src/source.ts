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
/** How the source view lays the text out, beyond writing it as it is. */
export interface SourceLayout {
  /**
   * Whether an element written with one attribute to a line reads as a property sheet: its name,
   * then a name and a value to a row, with no `=`, no quotes and no closing tag.
   */
  readonly sheets?: boolean;
}

export function renderSource(
  document: Document,
  index: TreeIndex,
  colors: readonly NxViewerColor[],
  layout: SourceLayout = {}
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
  const identifiers = identifierNodes(index);

  // A string its viewer formats as an identifier: one quiet token, its quotes inside it.
  const writeIdentifier = (element: HTMLElement, start: number, end: number): void => {
    element.classList.add("id-value");
    const quote = (): HTMLElement => {
      const mark = document.createElement("span");
      mark.className = "id-quote";
      mark.textContent = '"';
      return mark;
    };
    element.append(quote(), text.slice(start + 1, end - 1), quote());
  };

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
  const idPlans = elementIdPlans(index, startOf, endOf);

  // An element's id, moved to where its viewer puts it: its value alone, as one quiet token that
  // keeps the attribute's key and its value's.
  const idChip = (plan: ElementIdPlan): HTMLElement => {
    const attribute = tree.nodes[plan.attribute]!;
    const literal = tree.nodes[plan.literal]!;
    const chip = document.createElement("span");
    chip.className = plan.atEnd ? "n id-value id-part id-end" : "n id-value id-part";
    chip.dataset["role"] = attribute.role;
    chip.dataset["key"] = attribute.key;
    chip.title = attribute.name ?? "";
    const value = document.createElement("span");
    value.className = "n";
    value.dataset["role"] = literal.role;
    value.dataset["key"] = literal.key;
    value.textContent = plan.value;
    chip.append(value);
    return chip;
  };

  // The text from `from` to `to` around an element's children, less what its id plan takes out,
  // and with the id put in where the plan says.
  const writeGap = (parent: HTMLElement, from: number, to: number, plan: ElementIdPlan | undefined): void => {
    if (plan === undefined) {
      writeText(parent, from, to);
      return;
    }
    let at = from;
    while (at < to || (!plan.written && at === plan.insertAt && at === to)) {
      if (!plan.written && at === plan.insertAt) {
        parent.append(" ", idChip(plan));
        plan.written = true;
        // A chip last in its tag closes it with no space between: its padding is gap enough.
        if (plan.atEnd && text[at] === " " && /^\s*\/?>/.test(text.slice(at, at + 3))) {
          at += 1;
        }
        continue;
      }
      if (at >= plan.removeFrom && at < plan.removeTo) {
        at = Math.min(plan.removeTo, to);
        continue;
      }
      let next = to;
      for (const point of [plan.removeFrom, plan.written ? to : plan.insertAt]) {
        if (point > at && point < next) {
          next = point;
        }
      }
      writeText(parent, at, next);
      at = next;
    }
  };

  const writeNodes = (
    parent: HTMLElement,
    children: readonly number[],
    from: number,
    to: number,
    plan?: ElementIdPlan
  ): void => {
    let at = from;
    const ordered = [...children].sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
    for (const child of ordered) {
      const node = tree.nodes[child]!;
      const start = startOf(node);
      const end = endOf(node);
      if (start < at || end > to || end < start || child === plan?.attribute) {
        continue;
      }
      writeGap(parent, at, start, plan);
      if (layout.sheets === true && node.role === "element" && sheetable(child)) {
        parent.append(writeSheet(child, start, end));
        at = end;
        continue;
      }
      if (layout.sheets === true && node.role === "element" && oneLineable(child)) {
        parent.append(writeLine(child, start, end));
        at = end;
        continue;
      }
      const element = document.createElement("span");
      element.className = "n";
      element.dataset["role"] = node.role;
      element.dataset["key"] = node.key;
      // A comment is prose, which a host may set in another font than the code around it.
      if (node.role === "comment" || node.role === "docComment") {
        element.classList.add("prose");
      }
      const quoted = node.role === "literal" && end - start >= 2 && text[start] === '"' && text[end - 1] === '"';
      if (quoted && identifiers.has(child)) {
        writeIdentifier(element, start, end);
      } else if (quoted && uiText.has(child)) {
        writeUIText(element, start, end);
      } else {
        if (uiText.has(child)) {
          element.classList.add("uitext", "uitext-body");
        }
        writeNodes(element, index.children[child] ?? [], start, end, idPlans.get(child));
      }
      parent.append(element);
      at = end;
    }
    writeGap(parent, at, to, plan);
  };

  // ---- Property sheets ----------------------------------------------------------------------

  const childrenOf = (at: number): number[] =>
    [...(index.children[at] ?? [])].sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
  const isContent = (at: number): boolean => tree.nodes[at]!.flags?.includes("content") ?? false;
  const isNote = (at: number): boolean => {
    const role = tree.nodes[at]!.role;
    return role === "comment" || role === "docComment";
  };
  const onOneLine = (from: number, to: number): boolean => !text.slice(from, to).includes("\n");
  const startsItsLine = (offset: number): boolean => text.slice(text.lastIndexOf("\n", offset - 1) + 1, offset).trim() === "";

  /**
   * Whether an element reads as a sheet: the author put each of its attributes on a line of its own, and every value is one the sheet can show in a cell: a literal, an element, a
   * list of elements or literals, or anything else on one line. Content it holds must be elements.
   */
  const sheetable = (at: number): boolean => {
    const plan = idPlans.get(at);
    const children = childrenOf(at);
    const attributes = children.filter(
      (child) => tree.nodes[child]!.role === "attribute" && !isContent(child) && child !== plan?.attribute
    );
    if (attributes.length === 0 || !attributes.every((child) => startsItsLine(startOf(tree.nodes[child]!)))) {
      return false;
    }
    for (const child of children) {
      const node = tree.nodes[child]!;
      if (isNote(child) || child === plan?.attribute) {
        continue;
      }
      if (node.role !== "attribute") {
        return false;
      }
      const values = childrenOf(child).filter((value) => !isNote(value));
      if (isContent(child)) {
        if (!values.every((value) => tree.nodes[value]!.role === "element")) {
          return false;
        }
        continue;
      }
      if (values.length !== 1) {
        return false;
      }
      const value = tree.nodes[values[0]!]!;
      const simple =
        value.role === "literal" ||
        value.role === "element" ||
        (value.role === "sequence" &&
          childrenOf(values[0]!).every((item) => ["element", "literal", "comment", "docComment"].includes(tree.nodes[item]!.role))) ||
        onOneLine(startOf(value), endOf(value));
      if (!simple) {
        return false;
      }
    }
    return true;
  };

  const marked = (at: number, tag: "span", className: string): HTMLElement => {
    const node = tree.nodes[at]!;
    const element = document.createElement(tag);
    element.className = className === "" ? "n" : `n ${className}`;
    element.dataset["role"] = node.role;
    element.dataset["key"] = node.key;
    return element;
  };

  /** One attribute's value in its cell: a string without its quotes, a boolean as a box, or as written. */
  const writeValue = (cell: HTMLElement, attribute: number): void => {
    const values = childrenOf(attribute).filter((value) => !isNote(value));
    if (values.length !== 1) {
      const nodes = childrenOf(attribute);
      if (nodes.length > 0) {
        writeNodes(cell, nodes, startOf(tree.nodes[nodes[0]!]!), endOf(tree.nodes[nodes[nodes.length - 1]!]!));
      }
      return;
    }
    const at = values[0]!;
    const value = tree.nodes[at]!;
    const start = startOf(value);
    const end = endOf(value);
    const written = value.value ?? "";
    if (value.role === "literal" && written.startsWith('"') && end - start >= 2) {
      if (identifiers.has(at)) {
        const chip = marked(at, "span", "id-value");
        chip.textContent = text.slice(start + 1, end - 1);
        cell.append(chip);
      } else if (uiText.has(at)) {
        const words = marked(at, "span", "uitext");
        const block = document.createElement("span");
        block.className = "uitext-block";
        const body = document.createElement("span");
        body.className = "uitext-body";
        body.textContent = text.slice(start + 1, end - 1);
        block.append(body);
        words.append(block);
        cell.append(words);
      } else {
        const words = marked(at, "span", "");
        writeText(words, start + 1, end - 1);
        cell.append(words);
      }
      return;
    }
    if (value.role === "literal" && (written === "true" || written === "false")) {
      // A mark, not a box: a check box looks like something to click, and this only reads.
      const box = marked(at, "span", `sheet-boolean ${written}`);
      box.textContent = written === "true" ? "\u2713" : "\u2717";
      box.title = written;
      box.setAttribute("aria-label", written);
      cell.append(box);
      return;
    }
    if (value.role === "sequence") {
      // A list reads an item to a line, without the braces around it.
      const list = marked(at, "span", "sheet-list");
      writeItems(list, childrenOf(at));
      cell.append(list);
      return;
    }
    writeNodes(cell, [at], start, end);
  };

  /**
   * Whether an attribute's value is structure that reads under its name: a list, or an element shown
   * as a sheet. Text stays in the value column however many lines it takes.
   */
  const spansLines = (attribute: number): boolean => {
    const values = childrenOf(attribute).filter((value) => !isNote(value));
    if (values.length !== 1) {
      return false;
    }
    const value = tree.nodes[values[0]!]!;
    if (value.role === "sequence") {
      return true;
    }
    return value.role === "element" && sheetable(values[0]!);
  };

  /** An element as a property sheet: its name and id, then a row to an attribute, then what it holds. */
  const writeSheet = (at: number, start: number, end: number): HTMLElement => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const sheet = marked(at, "span", "sheet");
    sheet.style.setProperty("--indent", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
    const head = document.createElement("span");
    head.className = "sheet-head";
    const nameStart = start + 1;
    writeText(head, nameStart, nameStart + (node.name ?? "").length);
    if (plan !== undefined) {
      head.append(" ", idChip(plan));
      plan.written = true;
    }
    sheet.append(head);
    for (const child of childrenOf(at)) {
      const childNode = tree.nodes[child]!;
      if (child === plan?.attribute) {
        continue;
      }
      if (childNode.role === "attribute" && !isContent(child)) {
        const row = marked(child, "span", "sheet-row");
        const name = document.createElement("span");
        name.className = "sheet-name";
        const nameAt = startOf(childNode);
        writeText(name, nameAt, nameAt + (childNode.name ?? "").length);
        const cell = document.createElement("span");
        cell.className = "sheet-value";
        row.append(name, cell);
        sheet.append(row);
        // A value of several lines reads under its name, indented, and not in the value column.
        if (spansLines(child)) {
          const below = document.createElement("span");
          below.className = "sheet-below";
          writeValue(below, child);
          row.append(below);
        } else {
          writeValue(cell, child);
        }
      } else if (childNode.role === "attribute") {
        const held = marked(child, "span", "sheet-held");
        writeItems(held, childrenOf(child));
        sheet.append(held);
      } else {
        const line = document.createElement("span");
        line.className = "sheet-note";
        writeNodes(line, [child], startOf(childNode), endOf(childNode));
        sheet.append(line);
      }
    }
    return sheet;
  };

  // ---- Elements on one line ------------------------------------------------------------------

  /**
   * Whether an element written on one line reads as one: its name, then its properties, each a name,
   * an `=` and a value, with no angle brackets, no quotes and no closing `/>`. It must set only
   * properties, with nothing it holds and no comment in its tag.
   */
  const oneLineable = (at: number): boolean => {
    const node = tree.nodes[at]!;
    if (!onOneLine(startOf(node), endOf(node))) {
      return false;
    }
    return childrenOf(at).every((child) => {
      const candidate = tree.nodes[child]!;
      return (
        candidate.role === "attribute" &&
        !isContent(child) &&
        childrenOf(child).filter((value) => !isNote(value)).length === 1 &&
        childrenOf(child).every((value) => !isNote(value))
      );
    });
  };

  /** A property's value on an element's line: as a sheet shows it, or as written with its braces. */
  const writeLineValue = (parent: HTMLElement, attribute: number): void => {
    const at = childrenOf(attribute)[0]!;
    const value = tree.nodes[at]!;
    const written = value.value ?? "";
    if (value.role === "literal" && (written.startsWith('"') || written === "true" || written === "false")) {
      writeValue(parent, attribute);
      return;
    }
    if (value.role === "element" && oneLineable(at)) {
      parent.append(writeLine(at, startOf(value), endOf(value)));
      return;
    }
    // Anything else, braces and all, as it is written after the `=`.
    const attributeNode = tree.nodes[attribute]!;
    const afterEquals = text.indexOf("=", startOf(attributeNode)) + 1;
    writeNodes(parent, [at], afterEquals, endOf(attributeNode));
  };

  /** One piece of an element's line: what column it belongs to when lines are aligned, and its DOM. */
  interface LinePart {
    readonly key: string;
    readonly element: HTMLElement;
  }

  /** The attribute whose string the element's viewer names as its title, if it sets one. */
  const titleOf = (at: number): number | undefined => {
    const node = tree.nodes[at]!;
    const viewer = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.viewer;
    return childrenOf(at).find((child) => {
      const candidate = tree.nodes[child]!;
      const values = childrenOf(child);
      return (
        candidate.name === viewer?.title &&
        child !== idPlans.get(at)?.attribute &&
        values.length === 1 &&
        tree.nodes[values[0]!]!.role === "literal" &&
        (tree.nodes[values[0]!]!.value ?? "").startsWith('"')
      );
    });
  };

  /** The properties of a one-line element shown by name: all but its id and its title. */
  const namedOf = (at: number): number[] => {
    const title = titleOf(at);
    return childrenOf(at).filter((child) => child !== idPlans.get(at)?.attribute && child !== title);
  };

  /** The pieces of an element's line, in reading order: name, id, title, properties, id. */
  const lineParts = (at: number, start: number): LinePart[] => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const parts: LinePart[] = [];
    const kind = document.createElement("span");
    kind.className = "line-kind";
    writeText(kind, start + 1, start + 1 + (node.name ?? "").length);
    parts.push({ key: "kind", element: kind });
    if (plan !== undefined && !plan.atEnd) {
      parts.push({ key: "id-start", element: idChip(plan) });
    }
    const title = titleOf(at);
    if (title !== undefined) {
      const part = marked(title, "span", "line-property");
      writeLineValue(part, title);
      parts.push({ key: "title", element: part });
    }
    for (const child of namedOf(at)) {
      const part = marked(child, "span", "line-property");
      const name = document.createElement("span");
      name.className = "line-name";
      const nameAt = startOf(tree.nodes[child]!);
      writeText(name, nameAt, nameAt + (tree.nodes[child]!.name ?? "").length);
      // On a line, a named property keeps its `=`: it ties the name to a value that may hold
      // spaces, where a sheet's columns do that for it.
      const equals = document.createElement("span");
      equals.className = "line-equals";
      equals.textContent = "=";
      part.append(name, equals);
      writeLineValue(part, child);
      parts.push({ key: `property:${tree.nodes[child]!.name ?? ""}`, element: part });
    }
    if (plan !== undefined && plan.atEnd) {
      parts.push({ key: "id-end", element: idChip(plan) });
    }
    if (plan !== undefined) {
      plan.written = true;
    }
    return parts;
  };

  /** Lays parts out one after another, with a space after the name before an id and two otherwise. */
  const flow = (parent: HTMLElement, parts: readonly LinePart[]): void => {
    parts.forEach((part, position) => {
      if (position > 0) {
        const gap = document.createElement("span");
        gap.className = "line-gap";
        gap.textContent = position === 1 && part.key === "id-start" ? " " : "  ";
        parent.append(gap);
      }
      parent.append(part.element);
    });
  };

  /** An element on one line: its name and id, its title's value alone, then its other properties. */
  const writeLine = (at: number, start: number, end: number): HTMLElement => {
    const line = marked(at, "span", "line-element");
    flow(line, lineParts(at, start));
    void end;
    return line;
  };

  /**
   * How a list of elements of one kind, each on one line, lines up: a column to each piece any of
   * them has, by property name, when they mostly share their properties; when they do not, only
   * their names and titles line up. Nothing when the list is not such a list.
   */
  const alignment = (items: readonly number[]): { columns: string[]; full: boolean } | undefined => {
    if (items.length < 2) {
      return undefined;
    }
    const kinds = new Set(items.map((item) => tree.nodes[item]!.role === "element" ? tree.nodes[item]!.name : undefined));
    if (kinds.size !== 1 || kinds.has(undefined) || !items.every((item) => oneLineable(item) && !sheetable(item))) {
      return undefined;
    }
    const first = tree.nodes[items[0]!]!;
    const viewer = first.declaration === undefined ? undefined : tree.declarations[first.declaration]?.viewer;
    const order = viewer?.properties?.map((property) => property.name) ?? [];
    const named = items.map((item) => namedOf(item).map((child) => tree.nodes[child]!.name ?? ""));
    const used: string[] = [];
    for (const names of named) {
      for (const name of names) {
        if (!used.includes(name)) {
          used.push(name);
        }
      }
    }
    used.sort((a, b) => {
      const ia = order.indexOf(a);
      const ib = order.indexOf(b);
      return ia < 0 || ib < 0 ? 0 : ia - ib;
    });
    // One row far wider than the rest in some column would push that column, and all after it, far
    // from the rows it belongs to: such a list is left to flow.
    const widths = new Map<number, number[]>();
    for (const item of items) {
      const title = titleOf(item);
      for (const child of [...(title === undefined ? [] : [title]), ...namedOf(item)]) {
        const key = child === title ? -1 : used.indexOf(tree.nodes[child]!.name ?? "");
        const node = tree.nodes[child]!;
        const width = child === title ? endOf(node) - startOf(node) - (node.name ?? "").length - 3 : endOf(node) - startOf(node);
        widths.set(key, [...(widths.get(key) ?? []), width]);
      }
    }
    for (const column of widths.values()) {
      const sorted = [...column].sort((a, b) => a - b);
      const median = sorted[Math.floor((sorted.length - 1) / 2)]!;
      const widest = sorted[sorted.length - 1]!;
      if (widest > median * OUTLIER_RATIO && widest - median > OUTLIER_CHARACTERS) {
        return undefined;
      }
    }
    const filled = named.reduce((sum, names) => sum + names.length, 0);
    // Columns pay their way when most rows fill most of them; otherwise they would be mostly gaps.
    const full = used.length === 0 || filled / (used.length * items.length) >= 0.6;
    const plans = items.map((item) => idPlans.get(item));
    const columns = ["kind"];
    if (plans.some((plan) => plan !== undefined && !plan.atEnd)) {
      columns.push("id-start");
    }
    if (items.some((item) => titleOf(item) !== undefined)) {
      columns.push("title");
    }
    if (full) {
      columns.push(...used.map((name) => `property:${name}`));
      if (plans.some((plan) => plan?.atEnd === true)) {
        columns.push("id-end");
      }
    } else {
      columns.push("rest");
    }
    return { columns, full };
  };

  /**
   * The items of a list or of what an element holds, a line to each. A run of one-line elements of
   * one kind lines up in columns, as a table does, with a gap where one lacks a property the others
   * set.
   */
  const writeItems = (parent: HTMLElement, items: readonly number[]): void => {
    const aligned = items.some((item) => isNote(item)) ? undefined : alignment(items);
    if (aligned === undefined) {
      for (const item of items) {
        const line = document.createElement("span");
        line.className = "sheet-item";
        writeNodes(line, [item], startOf(tree.nodes[item]!), endOf(tree.nodes[item]!));
        parent.append(line);
      }
      return;
    }
    const table = document.createElement("span");
    table.className = "line-table";
    table.style.gridTemplateColumns = `repeat(${aligned.columns.length}, max-content)`;
    for (const item of items) {
      const node = tree.nodes[item]!;
      const line = marked(item, "span", "line-element line-row");
      const parts = lineParts(item, startOf(node));
      const rest: LinePart[] = [];
      for (const part of parts) {
        const column = aligned.columns.indexOf(part.key);
        if (column < 0) {
          rest.push(part);
          continue;
        }
        part.element.style.gridColumn = String(column + 1);
        line.append(part.element);
      }
      if (rest.length > 0) {
        // What does not line up flows in the last column, as it would on its own.
        const flowing = document.createElement("span");
        flowing.className = "line-rest";
        flowing.style.gridColumn = String(aligned.columns.length);
        flow(flowing, rest);
        line.append(flowing);
      }
      table.append(line);
    }
    parent.append(table);
  };

  const top = tree.nodes.flatMap((node, at) => (node.parent === undefined ? [at] : []));
  writeNodes(root, top, 0, text.length);
  markIndentation(document, root);
  return root;
}

/**
 * Where an element's id goes: the attribute that holds it and the string it holds, the text taken
 * out for it, and the offset it is put in at, as a space and a chip.
 */
interface ElementIdPlan {
  readonly attribute: number;
  readonly literal: number;
  readonly value: string;
  readonly removeFrom: number;
  readonly removeTo: number;
  readonly insertAt: number;
  /** Whether the id goes last in its tag, not right after the element's name. */
  readonly atEnd: boolean;
  written: boolean;
}

/**
 * The plan for each element whose viewer names an id property it sets to a string: the attribute
 * comes out of its place, with its own line when it stands alone on one, and its value reads right
 * after the element's name, or last in its tag, after the attributes that stay.
 */
function elementIdPlans(
  index: TreeIndex,
  startOf: (node: SourceNode) => number,
  endOf: (node: SourceNode) => number
): Map<number, ElementIdPlan> {
  const { tree, text } = index;
  const plans = new Map<number, ElementIdPlan>();
  tree.nodes.forEach((node, at) => {
    if (node.role !== "element" || node.declaration === undefined) {
      return;
    }
    const viewer = tree.declarations[node.declaration]?.viewer;
    if (viewer?.id === undefined) {
      return;
    }
    const children = index.children[at] ?? [];
    const attributes = children.filter((child) => {
      const candidate = tree.nodes[child]!;
      return candidate.role === "attribute" && !(candidate.flags?.includes("content") ?? false);
    });
    const attribute = attributes.find((child) => tree.nodes[child]!.name === viewer.id);
    if (attribute === undefined) {
      return;
    }
    const values = (index.children[attribute] ?? []).filter((child) => tree.nodes[child]!.role !== "comment");
    const literal = values.length === 1 ? values[0]! : undefined;
    const literalNode = literal === undefined ? undefined : tree.nodes[literal]!;
    if (literal === undefined || literalNode?.role !== "literal" || !(literalNode.value ?? "").startsWith('"')) {
      return;
    }
    const start = startOf(tree.nodes[attribute]!);
    const end = endOf(tree.nodes[attribute]!);
    // An attribute alone on its line takes the line with it; one beside others takes the space before it.
    const lineStart = text.lastIndexOf("\n", start - 1) + 1;
    const lineEnd = text.indexOf("\n", end);
    const alone = text.slice(lineStart, start).trim() === "" && text.slice(end, lineEnd < 0 ? text.length : lineEnd).trim() === "";
    let removeFrom = start;
    let removeTo = end;
    if (alone && lineEnd >= 0) {
      removeFrom = lineStart;
      removeTo = lineEnd + 1;
    } else {
      while (removeFrom > 0 && (text[removeFrom - 1] === " " || text[removeFrom - 1] === "\t")) {
        removeFrom -= 1;
      }
    }
    const elementStart = startOf(node);
    let nameEnd = elementStart + 1 + (node.name ?? "").length;
    if (text[nameEnd] === ":") {
      // A text type after the name, `<Note:markdown`, belongs with the name.
      nameEnd += 1;
      while (nameEnd < text.length && /[A-Za-z0-9_.-]/.test(text[nameEnd]!)) {
        nameEnd += 1;
      }
    }
    const others = attributes.filter((child) => child !== attribute);
    const atEnd = viewer.idPosition === "end" && others.length > 0;
    const insertAt = atEnd ? Math.max(...others.map((child) => endOf(tree.nodes[child]!))) : nameEnd;
    plans.set(at, {
      attribute,
      literal,
      value: (literalNode.value ?? "").slice(1, -1),
      removeFrom,
      removeTo,
      insertAt,
      atEnd,
      written: false,
    });
  });
  return plans;
}

/**
 * How much wider than the middle row of a column its widest row may be before a list is left
 * unaligned: both a ratio and a number of characters must be passed.
 */
const OUTLIER_RATIO = 1.5;
const OUTLIER_CHARACTERS = 12;

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
 * The string literals an attribute holds directly when its property's viewer formats it as an
 * identifier: `<PropertyViewer of=id format=id />`, or the viewer of an alias its type names.
 */
function identifierNodes(index: TreeIndex): Set<number> {
  const { tree } = index;
  const found = new Set<number>();
  tree.nodes.forEach((node, at) => {
    if (node.role !== "literal" || !(node.value ?? "").startsWith('"') || node.parent === undefined) {
      return;
    }
    const attribute = tree.nodes[node.parent]!;
    const element = attribute.parent === undefined ? undefined : tree.nodes[attribute.parent];
    if (attribute.role !== "attribute" || element?.declaration === undefined) {
      return;
    }
    const viewer = tree.declarations[element.declaration]?.viewer;
    if (viewer?.properties?.find((property) => property.name === attribute.name)?.format === "id") {
      found.add(at);
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
