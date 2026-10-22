/**
 * The source reading: the document's text with each node of the source tree wrapped in a span that
 * carries its key. Code reads as written; an element reads as its name and properties on one line
 * when they fit there, and as a group, a row to each property, when they do not or when it holds
 * items, without tags or quotes, with its id moved to where its visualizer puts it and its booleans
 * read as marks. Colors come from the host, as ranges over the
 * text, so a host colors the reading with the highlighter and themes it colors its editor with.
 */
import type { SourceDeclarationKind, SourceNode, SourceValueFormat } from "@nx-lang/language-protocol";

import { isElementFunction, VALUE_ROLES } from "./card.js";
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

/** What the reading draws with, beyond the text and its colors. */
export interface SourceLayout {
  /**
   * The keys of the elements whose properties do not fit on one line in the pane, which give each
   * property a row. The element measures which those are, from the reading drawn without them.
   */
  readonly wrapped?: ReadonlySet<string> | undefined;
  /**
   * Draws the icon of the given name, from the Lucide set, or nothing when it has none. With it, an
   * element on lines of its own starts with an icon: the one its visualizer names, or else one for
   * its kind of declaration. Without it, no element has an icon.
   */
  readonly icons?: NxViewerIcons | undefined;
  /**
   * The keys of the groups and declarations shown folded: an element over several lines as its
   * header alone, and a declaration over several lines as its first line.
   */
  readonly folded?: ReadonlySet<string> | undefined;
  /** The fold just toggled and whether it was folded, so its chevron is drawn where it was. */
  readonly turning?: { readonly key: string; readonly wasFolded: boolean } | undefined;
}

/** Draws an icon by its Lucide name: an element the viewer copies, or nothing. */
export type NxViewerIcons = (name: string) => Element | undefined;

/** The icon an element starts with when its visualizer names none, by its kind of declaration. */
const KIND_ICONS: Partial<Record<SourceDeclarationKind, string>> = {
  component: "component",
  record: "box",
  action: "zap",
  union: "tag",
  function: "square-function",
};

/** The slots of an `if`, a match or a `for` that hold its test, subject, pattern or list. */
const BLOCK_HEADS = new Set(["test", "subject", "pattern", "in"]);

/** A run of top-level declarations that support the document, folded together to one line. */
export interface ReadingRun {
  /** The run's fold key: `run:` and the key of its first node. */
  readonly key: string;
  /** Its top-level nodes, comments included, in source order. */
  readonly nodes: readonly number[];
  /** What it holds, in words: "6 types · 2 visualizers". */
  readonly summary: string;
}

/** What a supporting top-level node is called in a run's summary, or nothing for a primary one. */
function supportingKind(index: TreeIndex, at: number): [string, string] | undefined {
  const { tree } = index;
  const node = tree.nodes[at]!;
  if (node.role === "import") {
    return ["import", "imports"];
  }
  if (node.role !== "declaration") {
    return undefined;
  }
  const flags = node.flags ?? [];
  if (flags.includes("visualizer")) {
    return ["visualizer", "visualizers"];
  }
  const kind = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.kind;
  switch (kind) {
    case "record":
    case "union":
    case "alias":
      return ["type", "types"];
    case "action":
      return ["action", "actions"];
    case "component":
      // A component with no body of its own, one the host draws or an abstract base, is part of a
      // design system; one with a body is the document's.
      return flags.includes("external") || flags.includes("abstract") ? ["component", "components"] : undefined;
    case "function": {
      // An element function (`let <Name …>`, whatever modifiers come before it) is used as a
      // component is, and `root` is what the document returns.
      return node.name === "root" || isElementFunction(index, at) ? undefined : ["function", "functions"];
    }
    default:
      return undefined;
  }
}

/**
 * The runs of top-level declarations that support the document rather than make it up: imports,
 * types, actions, components with no body of their own, visualizer declarations and functions,
 * apart from `root` and element functions. A comment goes with the declaration it is written over, unless a
 * blank line sets it apart; values, components with bodies and what the document returns end a run.
 */
export function readingRuns(index: TreeIndex): ReadingRun[] {
  const { tree, text } = index;
  const offsets = utf16Offsets(text, index.bytes.length);
  const startOf = (at: number): number => offsets[Math.min(tree.nodes[at]!.range.startByte, index.bytes.length)]!;
  const endOf = (at: number): number => offsets[Math.min(tree.nodes[at]!.range.endByte, index.bytes.length)]!;
  const top = tree.nodes
    .flatMap((node, at) => (node.parent === undefined ? [at] : []))
    .sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
  const isNote = (at: number): boolean => ["comment", "docComment"].includes(tree.nodes[at]!.role);
  // Each node's side: a declaration's own, and a comment's the next declaration's when nothing but a
  // line break stands between them.
  const supporting = new Map<number, [string, string] | undefined>();
  for (let i = top.length - 1; i >= 0; i--) {
    const at = top[i]!;
    if (!isNote(at)) {
      supporting.set(at, supportingKind(index, at));
      continue;
    }
    const next = top[i + 1];
    const between = next === undefined ? "\n\n" : text.slice(endOf(at), startOf(next));
    supporting.set(at, next !== undefined && !/\n[ \t]*\n/.test(between) && supporting.get(next) !== undefined ? ["", ""] : undefined);
  }
  const runs: ReadingRun[] = [];
  let current: number[] = [];
  const close = (): void => {
    const counts = new Map<string, [number, string, string]>();
    for (const at of current) {
      const kind = supporting.get(at)!;
      if (kind[0] !== "") {
        const counted = counts.get(kind[0]) ?? [0, kind[0], kind[1]];
        counted[0] += 1;
        counts.set(kind[0], counted);
      }
    }
    // A run is worth folding when it holds more than one declaration.
    const total = [...counts.values()].reduce((sum, [count]) => sum + count, 0);
    if (total > 1) {
      const summary = [...counts.values()].map(([count, one, many]) => `${count} ${count === 1 ? one : many}`).join(" · ");
      runs.push({ key: `run:${tree.nodes[current[0]!]!.key}`, nodes: current, summary });
    }
    current = [];
  };
  for (const at of top) {
    if (supporting.get(at) !== undefined) {
      current.push(at);
    } else {
      close();
    }
  }
  close();
  return runs;
}

/**
 * What the reading folds until the reader says otherwise: each run of supporting declarations, and
 * each of those declarations, so opening a run shows them a line each.
 */
export function defaultFolded(index: TreeIndex): string[] {
  const keys: string[] = [];
  for (const run of readingRuns(index)) {
    keys.push(run.key);
    for (const at of run.nodes) {
      const node = index.tree.nodes[at]!;
      if (node.role === "declaration") {
        keys.push(node.key);
      }
    }
  }
  return keys;
}

/**
 * Renders the document's text as the reading shows it, colored by `colors`, with its nodes marked:
 * an element as a line of its name and properties, or as a group with no tags when it holds items
 * or is in `layout.wrapped`, and everything else as written.
 */
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

  // A piece of the text in one color, or in none: a text node, or a span when it has a color. It
  // shows the text from `from` to `to`, or `shown` in its place.
  const piece = (
    parent: HTMLElement,
    from: number,
    to: number,
    range: NxViewerColor | undefined,
    shown = text.slice(from, to)
  ): void => {
    if (range === undefined) {
      parent.append(shown);
      return;
    }
    const element = document.createElement("span");
    element.className = "t";
    if (range.light !== undefined) {
      element.style.setProperty("--l", range.light);
    }
    if (range.dark !== undefined) {
      element.style.setProperty("--d", range.dark);
    }
    const style = range.fontStyle ?? 0;
    if ((style & 1) !== 0) {
      element.style.fontStyle = "italic";
    }
    if ((style & 2) !== 0) {
      element.style.fontWeight = "bold";
    }
    if ((style & 4) !== 0) {
      element.style.textDecoration = "underline";
    }
    element.textContent = shown;
    parent.append(element);
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

  // A string literal's words as a person reads them: `\"` and `\\` as a quote and a backslash, as its
  // editor shows them. Any other escape, as `\n`, leaves the words as written.
  const readable = (start: number, end: number): string =>
    decodeString(text.slice(start, end)) ?? text.slice(start + 1, end - 1);
  // A string literal from `start` to `end`, quotes and all, written in the color the text has there,
  // with its escapes read; or nothing, when it has none to read, so it is written as it is.
  const writeReadable = (parent: HTMLElement, start: number, end: number, quoted: boolean): boolean => {
    const written = text.slice(start + 1, end - 1);
    const words = readable(start, end);
    if (words === written) {
      return false;
    }
    const range = colors.find((candidate) => candidate.start <= start + 1 && candidate.end > start + 1);
    piece(parent, start, end, range, quoted ? `"${words}"` : words);
    return true;
  };

  const uiText = uiTextNodes(index);
  const formats = formattedNodes(index);
  const identifiers = new Set([...formats].flatMap(([at, format]) => (format === "id" ? [at] : [])));

  // A small square of the color a string names, before it, for a string its visualizer formats as a
  // color. Nothing when the string names no color: the square would show a color that is not there.
  const swatch = (at: number, start: number, end: number): HTMLElement | undefined => {
    if (formats.get(at) !== "color") {
      return undefined;
    }
    const square = document.createElement("span");
    square.className = "swatch";
    // A color, never anything else: `background-color` takes no URL.
    square.style.backgroundColor = text.slice(start + 1, end - 1).trim();
    if (square.style.backgroundColor === "") {
      return undefined;
    }
    square.setAttribute("aria-hidden", "true");
    return square;
  };

  // A string its visualizer formats as an identifier: one quiet token, its quotes inside it.
  const writeIdentifier = (element: HTMLElement, start: number, end: number): void => {
    element.classList.add("id-value");
    const quote = (): HTMLElement => {
      const mark = document.createElement("span");
      mark.className = "id-quote";
      mark.textContent = '"';
      return mark;
    };
    element.append(quote(), readable(start, end), quote());
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
    // How far into its line the words begin, which is what the box may not be wider than the pane
    // by.
    const lineStart = text.lastIndexOf("\n", start) + 1;
    block.style.setProperty("--column", String(start + 1 - lineStart));
    const body = document.createElement("span");
    body.className = "uitext-body";
    body.append(readable(start, end), quote());
    block.append(body);
    element.append(quote(), block);
  };

  // Writes the nodes at `children` and the text between them, from `from` to `to`. A node whose
  // range does not sit inside what is left to write is left unmarked rather than written twice.
  const idPlans = elementIdPlans(index, startOf, endOf);

  // What the pointer rests on to see the card of the node at `at`, as a name or a value.
  const hoverable = (element: HTMLElement, at: number): void => {
    element.dataset["hover"] = tree.nodes[at]!.key;
  };

  // An element's id, moved to where its visualizer puts it: its value alone, as one quiet token
  // that keeps the attribute's key and its value's. Its card is its property's.
  const idChip = (plan: ElementIdPlan): HTMLElement => {
    const attribute = tree.nodes[plan.attribute]!;
    const literal = tree.nodes[plan.literal]!;
    const chip = document.createElement("span");
    chip.className = plan.atEnd ? "n id-value id-part id-end" : "n id-value id-part";
    chip.dataset["role"] = attribute.role;
    chip.dataset["key"] = attribute.key;
    hoverable(chip, plan.attribute);
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
      if (node.role === "element" && shapeOf(child) === "group") {
        parent.append(writeSheet(child, start, end));
        at = end;
        continue;
      }
      if (node.role === "element" && shapeOf(child) === "line") {
        parent.append(writeLine(child, start, end));
        at = end;
        continue;
      }
      const element = document.createElement("span");
      element.className = "n";
      element.dataset["role"] = node.role;
      element.dataset["key"] = node.key;
      // A value shows its card: a case, a reference, a member or a call. A qualified case's union is
      // the case's.
      const parentRole = node.parent === undefined ? undefined : tree.nodes[node.parent]!.role;
      if (VALUE_ROLES.has(node.role) && parentRole !== "case") {
        hoverable(element, child);
      }
      // A declaration over several lines folds to its first line.
      if (node.parent === undefined && node.role === "declaration" && !onOneLine(start, end)) {
        const folded = layout.folded?.has(node.key) === true;
        element.classList.add("fold-declaration");
        element.append(foldToggle(node.key, folded));
        if (folded) {
          element.classList.add("folded");
          const lineEnd = text.indexOf("\n", start);
          writeText(element, start, lineEnd);
          element.append(foldSummary(" …"));
          parent.append(element);
          at = end;
          continue;
        }
      }
      // An element written as it is, tag and all, starting a line of its own, starts with its icon
      // like a sheet or a one-line element does.
      const marker = node.role === "element" && startsItsLine(start) ? elementIcon(child) : undefined;
      if (marker !== undefined) {
        // It is a unit of the reading too, selected and hovered as a whole like a one-line element.
        element.classList.add("tag-line");
        element.style.setProperty("--line-column", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
        // Over several lines, its later lines keep the indentation the source gives them, as the
        // lines of the item it is in do.
        if (!onOneLine(start, end)) {
          element.classList.add("over-lines");
        }
        element.append(marker);
      }
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
        const square = quoted ? swatch(child, start, end) : undefined;
        if (square !== undefined) {
          element.append(square);
        }
        if (!(quoted && writeReadable(element, start, end, true))) {
          writeNodes(element, index.children[child] ?? [], start, end, idPlans.get(child));
        }
      }
      parent.append(element);
      at = end;
    }
    writeGap(parent, at, to, plan);
  };

  // ---- Property sheets ----------------------------------------------------------------------

  /**
   * The icon an element on lines of its own starts with, labelled with its name for a reader who
   * cannot see it, and showing the element's card to the pointer. Nothing when the host draws no
   * icons.
   */
  const elementIcon = (at: number): HTMLElement | undefined => {
    if (layout.icons === undefined) {
      return undefined;
    }
    const node = tree.nodes[at]!;
    const declaration = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
    const draw = (name: string | undefined): Element | undefined => (name === undefined ? undefined : layout.icons?.(name));
    // An icon the host cannot draw, as a name Lucide does not have, gives way to the kind's.
    const drawn = draw(declaration?.visualizer?.icon) ?? draw(declaration === undefined ? undefined : KIND_ICONS[declaration.kind]);
    if (drawn === undefined) {
      return undefined;
    }
    const icon = document.createElement("span");
    icon.className = "element-icon";
    icon.setAttribute("role", "img");
    icon.setAttribute("aria-label", node.name ?? "");
    hoverable(icon, at);
    icon.append(drawn.cloneNode(true));
    return icon;
  };

  const childrenOf = (at: number): number[] =>
    [...(index.children[at] ?? [])].sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
  const isContent = (at: number): boolean => tree.nodes[at]!.flags?.includes("content") ?? false;
  const isNote = (at: number): boolean => {
    const role = tree.nodes[at]!.role;
    return role === "comment" || role === "docComment";
  };
  const onOneLine = (from: number, to: number): boolean => !text.slice(from, to).includes("\n");
  const startsItsLine = (offset: number): boolean => text.slice(text.lastIndexOf("\n", offset - 1) + 1, offset).trim() === "";

  // ---- How each element reads ------------------------------------------------------------
  //
  // An element reads as a line, its name and then its properties, when they fit on one line in the
  // pane, and as a group, a row to each property, when they do not, however the source breaks its
  // lines. An element that holds items is a group either way, with its properties on its header
  // when they fit there. A property's element value reads on its row or its line, as a line, or as
  // a group under the property's name when it holds items or does not fit on its row.

  /** The element whose property's value the element at `at` is, or nothing for an item. */
  const holderOf = (at: number): number | undefined => {
    const parent = tree.nodes[at]!.parent;
    return parent !== undefined && tree.nodes[parent]!.role === "attribute" && !isContent(parent)
      ? tree.nodes[parent]!.parent
      : undefined;
  };
  /**
   * Whether an element's properties take a row each: they were measured not to fit on one line. A
   * property's element value is measured too, so one too long for its row reads as a group under
   * the property's name, and a short one, as `<Thickness …/>`, stays on its row.
   */
  const isWrapped = (at: number): boolean => layout.wrapped?.has(tree.nodes[at]!.key) === true;
  /** The properties an element shows by name or as its title: all but its id and what it holds. */
  const shownOf = (at: number): number[] =>
    childrenOf(at).filter(
      (child) => tree.nodes[child]!.role === "attribute" && !isContent(child) && child !== idPlans.get(at)?.attribute
    );
  /**
   * Marks an element whose line, or whose group's header, holds more than one property, for the
   * element to measure whether it fits on one line. One property is enough on a group's header when
   * the header is cut rather than wrapped, folded, or when that property is an element, which a row
   * lets fit on its own line.
   */
  const markFit = (element: HTMLElement, at: number): void => {
    const elementValue = (attribute: number): boolean => childrenOf(attribute).some((value) => tree.nodes[value]!.role === "element");
    const group = element.classList.contains("sheet");
    const least = group && (layout.folded?.has(tree.nodes[at]!.key) === true || shownOf(at).some(elementValue)) ? 1 : 2;
    // A property's element value is measured where a group shows it, on its header or its row. On a
    // line, its holder's line wraps as a whole, and a holder showing one property stays a line.
    const holder = holderOf(at);
    if (holder !== undefined && shapeOf(holder) !== "group") {
      return;
    }
    if (!isWrapped(at) && shownOf(at).length >= least) {
      element.classList.add("fit-check");
    }
  };

  const shapes = new Map<number, "group" | "line" | "written">();
  /** How an element reads: as a line, as a group, or as written. */
  const shapeOf = (at: number): "group" | "line" | "written" => {
    let shape = shapes.get(at);
    if (shape === undefined) {
      shape = lineable(at) ? "line" : sheetable(at) ? "group" : "written";
      shapes.set(at, shape);
    }
    return shape;
  };

  /**
   * Whether an attribute's value fits on a line after its name: one value, no comment, and an
   * element that reads as a line or anything else written on one line.
   */
  const inlineValue = (attribute: number): boolean => {
    const values = childrenOf(attribute);
    if (values.length !== 1 || isNote(values[0]!)) {
      return false;
    }
    const value = tree.nodes[values[0]!]!;
    if (value.role === "element") {
      return shapeOf(values[0]!) === "line";
    }
    return onOneLine(startOf(value), endOf(value));
  };

  /** Whether an attribute of a group reads on its header, after the name: its properties fit there. */
  const inHeader = (element: number, attribute: number): boolean => !isWrapped(element) && inlineValue(attribute);

  /**
   * Whether an element reads as a line: each property's value fits on a line, it holds no element
   * and no more than a line of text, and it has no comment in its tag. Measured not to fit, it
   * still reads as one when it shows one property at most, which a row would not shorten.
   */
  const lineable = (at: number): boolean => {
    for (const child of childrenOf(at)) {
      const candidate = tree.nodes[child]!;
      if (candidate.role !== "attribute") {
        return false;
      }
      if (isContent(child)) {
        // An element held reads under its holder, as an item of a group.
        const held = childrenOf(child);
        if (!onOneLine(startOf(candidate), endOf(candidate)) || held.some((item) => isNote(item) || tree.nodes[item]!.role === "element")) {
          return false;
        }
      } else if (!inlineValue(child)) {
        return false;
      }
    }
    return !isWrapped(at) || shownOf(at).length <= 1;
  };

  /**
   * Whether an element reads as a group: it shows a header (its icon, name, id and the properties
   * that fit there), a row to each other property, and under them what it holds, with no angle
   * brackets and no closing tag. Each row's value must be one a cell can show: a literal, an
   * element, a list of elements or literals, or anything else on one line. A comment inside an
   * attribute has no place in a row or on the header, so an element with one reads as written.
   */
  const sheetable = (at: number): boolean => {
    const plan = idPlans.get(at);
    for (const child of childrenOf(at)) {
      const childNode = tree.nodes[child]!;
      if (isNote(child) || child === plan?.attribute) {
        continue;
      }
      if (childNode.role !== "attribute") {
        return false;
      }
      if (isContent(child)) {
        continue;
      }
      const values = childrenOf(child);
      if (values.some(isNote)) {
        return false;
      }
      if (inHeader(at, child)) {
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
        chip.textContent = readable(start, end);
        cell.append(chip);
      } else if (uiText.has(at)) {
        const words = marked(at, "span", "uitext");
        const block = document.createElement("span");
        block.className = "uitext-block";
        const body = document.createElement("span");
        body.className = "uitext-body";
        body.textContent = readable(start, end);
        block.append(body);
        words.append(block);
        cell.append(words);
      } else {
        const words = marked(at, "span", "");
        const square = swatch(at, start, end);
        if (square !== undefined) {
          // The square and the color it shows stay together where a line wraps.
          words.classList.add("colored");
          words.append(square);
        }
        if (!writeReadable(words, start, end, false)) {
          writeText(words, start + 1, end - 1);
        }
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
   * Whether an attribute's value is structure that reads under its name: a list, or an element
   * shown as a sheet. Text stays in the value column however many lines it takes.
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
    return value.role === "element" && shapeOf(values[0]!) === "group";
  };

  /** An element as a property sheet: its name and id, then a row to an attribute, then what it holds. */
  /**
   * The control that folds or unfolds the unit with `key`: a chevron that hangs to the left of
   * where the unit starts, so it moves nothing.
   */
  const foldToggle = (key: string, folded: boolean): HTMLElement => {
    const toggle = document.createElement("span");
    toggle.className = folded ? "fold folded" : "fold";
    if (layout.turning?.key === key) {
      toggle.classList.add(layout.turning.wasFolded ? "was-folded" : "was-open");
    }
    toggle.dataset["foldKey"] = key;
    toggle.setAttribute("role", "button");
    toggle.setAttribute("aria-label", folded ? "Expand" : "Collapse");
    toggle.setAttribute("aria-expanded", String(!folded));
    return toggle;
  };

  /** A quiet note on a folded unit's line of what it hides. */
  const foldSummary = (words: string): HTMLElement => {
    const summary = document.createElement("span");
    summary.className = "fold-summary";
    summary.textContent = words;
    return summary;
  };

  /**
   * How many children a group's body writes, as the group shows them open: each element, and each
   * expression that gives elements, as `{Children}`, once; inside an `if`, a match or a `for`, what
   * every branch or its body writes, since all of them show. Text and comments are not children.
   */
  const childCount = (items: readonly number[]): number => {
    if (items.some((item) => tree.nodes[item]!.role === "text")) {
      return 0;
    }
    let count = 0;
    for (const item of items) {
      const role = tree.nodes[item]!.role;
      if (role === "condition" || role === "loop" || role === "match" || role === "matchArm") {
        // A block's test, subject, pattern, list or variables are not what it writes. A loop's
        // variables take their own names as their slots, so they are known by their role.
        count += childCount(
          childrenOf(item).filter((part) => tree.nodes[part]!.role !== "binding" && !BLOCK_HEADS.has(index.slots[part]!))
        );
      } else if (!isNote(item) && role !== "unparsed") {
        count += 1;
      }
    }
    return count;
  };

  /** "1 property", "3 children": a count and its noun. */
  const counted = (count: number, noun: string, nouns: string): string => `${count} ${count === 1 ? noun : nouns}`;

  const writeSheet = (at: number, start: number, end: number): HTMLElement => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const folded = layout.folded?.has(node.key) === true;
    const sheet = marked(at, "span", folded ? "sheet folded" : "sheet");
    markFit(sheet, at);
    sheet.style.setProperty("--indent", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
    const head = document.createElement("span");
    head.className = "sheet-head";
    head.append(foldToggle(node.key, folded));
    const marker = elementIcon(at);
    if (marker !== undefined) {
      head.append(marker);
    }
    const kind = document.createElement("span");
    kind.className = "sheet-kind";
    hoverable(kind, at);
    writeText(kind, start + 1, nameEnd(text, node, start));
    head.append(kind);
    if (plan !== undefined && !plan.atEnd) {
      head.append(" ", idChip(plan));
    }
    if (plan !== undefined) {
      plan.written = true;
    }
    sheet.append(head);
    // Folded, the group keeps its header and says what it hides: its title's value when its
    // visualizer names one, and otherwise how many properties and children it holds.
    let hiddenRows = 0;
    let hiddenChildren = 0;
    let hiddenTitle: string | undefined;
    let hiddenTitleAt: number | undefined;
    const visualizer = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.visualizer;
    for (const child of childrenOf(at)) {
      const childNode = tree.nodes[child]!;
      if (child === plan?.attribute) {
        continue;
      }
      if (childNode.role === "attribute" && !isContent(child) && inHeader(at, child)) {
        // On the header, after the name, as a one-line element shows it.
        const gap = document.createElement("span");
        gap.className = "line-gap";
        gap.textContent = "  ";
        head.append(gap, namedPart(child));
        continue;
      }
      if (folded) {
        if (childNode.role === "attribute" && isContent(child)) {
          hiddenChildren += childCount(childrenOf(child));
        } else if (childNode.role === "attribute") {
          hiddenRows += 1;
          const values = childrenOf(child);
          const value = values.length === 1 ? tree.nodes[values[0]!]! : undefined;
          if (childNode.name === visualizer?.title && value?.role === "literal" && (value.value ?? "").startsWith('"')) {
            hiddenTitle = text.slice(startOf(value) + 1, endOf(value) - 1);
            hiddenTitleAt = child;
          }
        }
        continue;
      }
      if (childNode.role === "attribute" && !isContent(child)) {
        const row = marked(child, "span", "sheet-row");
        const name = document.createElement("span");
        name.className = "sheet-name";
        hoverable(name, child);
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
    if (plan !== undefined && plan.atEnd) {
      // Last in its tag: after the attributes on the header, as a line puts it after its
      // properties.
      const gap = document.createElement("span");
      gap.className = "line-gap";
      gap.textContent = "  ";
      head.append(gap, idChip(plan));
    }
    if (folded) {
      const parts = hiddenTitle !== undefined
        ? [hiddenTitle]
        : [
            ...(hiddenRows > 0 ? [counted(hiddenRows, "property", "properties")] : []),
            ...(hiddenChildren > 0 ? [counted(hiddenChildren, "child", "children")] : []),
          ];
      if (parts.length === 0) {
        head.append(foldSummary(" …"));
      } else {
        // A title is words a person reads, set as text and cut to the line it has.
        head.append("  ");
        const summary = foldSummary(parts.join(" · "));
        summary.classList.add(hiddenTitle !== undefined ? "fold-title" : "fold-count");
        if (hiddenTitleAt !== undefined) {
          hoverable(summary, hiddenTitleAt);
        }
        head.append(summary);
      }
    }
    return sheet;
  };

  // ---- Elements on one line ------------------------------------------------------------------

  /** A property's value on an element's line: as a sheet shows it, or as written with its braces. */
  const writeLineValue = (parent: HTMLElement, attribute: number): void => {
    const at = childrenOf(attribute)[0]!;
    const value = tree.nodes[at]!;
    const written = value.value ?? "";
    if (value.role === "literal" && (written.startsWith('"') || written === "true" || written === "false")) {
      writeValue(parent, attribute);
      return;
    }
    if (value.role === "element" && shapeOf(at) === "line") {
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

  /** The attribute whose string the element's visualizer names as its title, if it sets one. */
  const titleOf = (at: number): number | undefined => {
    const node = tree.nodes[at]!;
    const visualizer = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.visualizer;
    return childrenOf(at).find((child) => {
      const candidate = tree.nodes[child]!;
      const values = childrenOf(child);
      return (
        candidate.name === visualizer?.title &&
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
    return childrenOf(at).filter((child) => child !== idPlans.get(at)?.attribute && child !== title && !isContent(child));
  };

  /**
   * A property on a line, by name: its name, an `=` and its value. On a line, a named property
   * keeps its `=`: it ties the name to a value that may hold spaces, where a sheet's columns do
   * that for it.
   */
  const namedPart = (attribute: number): HTMLElement => {
    const part = marked(attribute, "span", "line-property");
    const name = document.createElement("span");
    name.className = "line-name";
    hoverable(name, attribute);
    const nameAt = startOf(tree.nodes[attribute]!);
    writeText(name, nameAt, nameAt + (tree.nodes[attribute]!.name ?? "").length);
    const equals = document.createElement("span");
    equals.className = "line-equals";
    hoverable(equals, attribute);
    equals.textContent = "=";
    part.append(name, equals);
    writeLineValue(part, attribute);
    return part;
  };

  /** The pieces of an element's line, in reading order: name, id, title, properties, id. */
  const lineParts = (at: number, start: number, named = true): LinePart[] => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const parts: LinePart[] = [];
    if (named) {
      const kind = document.createElement("span");
      kind.className = "line-kind";
      hoverable(kind, at);
      writeText(kind, start + 1, nameEnd(text, node, start));
      parts.push({ key: "kind", element: kind });
    }
    if (plan !== undefined && !plan.atEnd) {
      parts.push({ key: "id-start", element: idChip(plan) });
    }
    const title = titleOf(at);
    if (title !== undefined) {
      // Shown without its name, the title shows its property's card.
      const part = marked(title, "span", "line-property");
      hoverable(part, title);
      writeLineValue(part, title);
      parts.push({ key: "title", element: part });
    }
    for (const child of namedOf(at)) {
      parts.push({ key: `property:${tree.nodes[child]!.name ?? ""}`, element: namedPart(child) });
    }
    for (const child of childrenOf(at).filter((candidate) => isContent(candidate))) {
      // What the element holds, as written between its tags: text, and expressions in braces.
      const held = marked(child, "span", "line-content");
      const content = tree.nodes[child]!;
      writeNodes(held, childrenOf(child), startOf(content), endOf(content));
      parts.push({ key: "content", element: held });
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

  /**
   * The type the place an element sits in declares, spelled without its `?`, `+` or `*`: the type
   * of the property or the declaration that holds it, through any list. Nothing when the place
   * declares none.
   */
  const slotType = (at: number): string | undefined => {
    let place = tree.nodes[at]!.parent;
    while (place !== undefined && tree.nodes[place]!.role === "sequence") {
      place = tree.nodes[place]!.parent;
    }
    if (place === undefined) {
      return undefined;
    }
    const holder = tree.nodes[place]!;
    let spelled: string | undefined;
    if (holder.role === "attribute") {
      const owner = holder.parent === undefined ? undefined : tree.nodes[holder.parent];
      const declared = owner?.declaration === undefined ? undefined : tree.declarations[owner.declaration];
      spelled = declared?.properties?.find((property) => property.name === holder.name)?.type;
    } else if (holder.role === "declaration") {
      const typed = (index.children[place] ?? []).find(
        (child) => tree.nodes[child]!.role === "typeReference" && index.slots[child] !== "extends"
      );
      spelled = typed === undefined ? undefined : tree.nodes[typed]!.value;
    }
    // A type spelled as an element, as `<PropertyVisualizer Target=Target/>+`, names its element.
    const trimmed = spelled?.replace(/[?+*]+$/, "").trim();
    return trimmed?.startsWith("<") ? /^<\s*([\w.]+)/.exec(trimmed)?.[1] : trimmed;
  };

  /** Whether nothing but spaces follows an offset on its line. */
  const endsItsLine = (offset: number): boolean => {
    const next = text.indexOf("\n", offset);
    return text.slice(offset, next < 0 ? text.length : next).trim() === "";
  };

  /**
   * An element on one line: its name and id, its title's value alone, then its other properties.
   * When it shares its line, with a property's name or anything else, it keeps the `<` and `/>` the
   * source writes, which show where it starts and ends; alone on its line, it needs neither.
   */
  const writeLine = (at: number, start: number, end: number): HTMLElement => {
    const line = marked(at, "span", "line-element");
    markFit(line, at);
    const alone = startsItsLine(start) && endsItsLine(end);
    const marker = alone ? elementIcon(at) : undefined;
    if (marker !== undefined) {
      line.append(marker);
    }
    if (alone) {
      line.classList.add("alone");
      line.style.setProperty("--line-column", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
    }
    if (!alone) {
      const open = document.createElement("span");
      open.className = "line-open";
      open.textContent = "<";
      line.append(open);
    }
    // An element of just the type its place declares needs no name: its icon stands for it, and
    // names it to the pointer and to a reader.
    const named = marker === undefined || slotType(at) !== tree.nodes[at]!.name;
    flow(line, lineParts(at, start, named));
    if (!alone) {
      const close = document.createElement("span");
      close.className = "line-close";
      // The `/` stands in its own span so the `>` can be drawn in close to it, as one mark.
      const slash = document.createElement("span");
      slash.className = "line-close-slash";
      slash.textContent = "/";
      close.append(slash, ">");
      line.append(close);
    }
    return line;
  };

  /**
   * The items of a list or of what an element holds, a line to each. Siblings do not line up with
   * each other: each reads on its own, as it is written.
   */
  const writeItems = (parent: HTMLElement, items: readonly number[]): void => {
    let previous: number | undefined;
    for (const item of items) {
      const node = tree.nodes[item]!;
      const start = startOf(node);
      const end = endOf(node);
      const line = document.createElement("span");
      line.className = "sheet-item";
      // A blank line the author left between two items keeps them apart, as a little room.
      if (previous !== undefined && /\n[ \t]*\n/.test(text.slice(previous, start))) {
        line.classList.add("after-blank");
      }
      // An item written as it is over several lines keeps the indentation of its later lines as the
      // source has it, measured from where the item starts, wherever the group puts it.
      if (!onOneLine(start, end) && !(node.role === "element" && shapeOf(item) !== "written")) {
        line.classList.add("as-written");
        line.style.setProperty("--column", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
        // Room for the item's own indentation on its first line, which its later lines share.
        const lead = document.createElement("span");
        lead.className = "as-written-lead";
        line.append(lead);
      }
      writeNodes(line, [item], start, end);
      if (line.classList.contains("as-written")) {
        // A block written as it is, as an `if` or a `for`, is a unit of the reading as a whole.
        line.querySelector(":scope > .n")?.classList.add("block-line");
      }
      parent.append(line);
      previous = end;
    }
  };

  const top = tree.nodes
    .flatMap((node, at) => (node.parent === undefined ? [at] : []))
    .sort((a, b) => tree.nodes[a]!.range.startByte - tree.nodes[b]!.range.startByte);
  // The document in segments: each run of supporting declarations, which may fold to one line, and
  // what stands between them as written.
  const runs = layout.folded === undefined ? [] : readingRuns(index);
  let from = 0;
  let next = 0;
  for (const run of runs) {
    const first = run.nodes[0]!;
    const last = run.nodes[run.nodes.length - 1]!;
    const runStart = startOf(tree.nodes[first]!);
    const before = top.slice(next, top.indexOf(first));
    writeNodes(root, before, from, runStart);
    // The run takes the blank lines after it, so folded it keeps the room the source gives it.
    const following = top[top.indexOf(last) + 1];
    const runEnd = following === undefined ? text.length : startOf(tree.nodes[following]!);
    const folded = layout.folded?.has(run.key) === true;
    const block = document.createElement("span");
    block.className = folded ? "fold-run folded" : "fold-run";
    const head = document.createElement("span");
    head.className = "fold-run-head";
    head.append(foldToggle(run.key, folded), foldSummary(run.summary));
    if (folded) {
      block.append(head, text.slice(endOf(tree.nodes[last]!), runEnd).includes("\n\n") ? "\n\n" : "\n");
    } else {
      // Open, the run keeps a quiet line above its declarations to fold them again by.
      block.append(head, "\n");
      writeNodes(block, run.nodes, runStart, runEnd);
    }
    root.append(block);
    from = runEnd;
    next = top.indexOf(last) + 1;
  }
  writeNodes(root, top.slice(next), from, text.length);
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
 * The plan for each element whose visualizer names an id property it sets to a string: the
 * attribute comes out of its place, with its own line when it stands alone on one, and its value
 * reads right after the element's name, or last in its tag, after the attributes that stay.
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
    const visualizer = tree.declarations[node.declaration]?.visualizer;
    if (visualizer?.id === undefined) {
      return;
    }
    const children = index.children[at] ?? [];
    const attributes = children.filter((child) => {
      const candidate = tree.nodes[child]!;
      return candidate.role === "attribute" && !(candidate.flags?.includes("content") ?? false);
    });
    const attribute = attributes.find((child) => tree.nodes[child]!.name === visualizer.id);
    if (attribute === undefined) {
      return;
    }
    // An attribute that holds a comment as well stays where it is, so the comment keeps its place.
    const values = index.children[attribute] ?? [];
    const literal = values.length === 1 ? values[0]! : undefined;
    const literalNode = literal === undefined ? undefined : tree.nodes[literal]!;
    if (literal === undefined || literalNode?.role !== "literal" || !(literalNode.value ?? "").startsWith('"')) {
      return;
    }
    const start = startOf(tree.nodes[attribute]!);
    const end = endOf(tree.nodes[attribute]!);
    // An attribute alone on its line takes the line with it; one beside others takes the space
    // before it.
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
    const others = attributes.filter((child) => child !== attribute);
    const atEnd = visualizer.idPosition === "end" && others.length > 0;
    const insertAt = atEnd ? Math.max(...others.map((child) => endOf(tree.nodes[child]!))) : nameEnd(text, node, startOf(node));
    plans.set(at, {
      attribute,
      literal,
      value: decodeString(literalNode.value ?? "") ?? (literalNode.value ?? "").slice(1, -1),
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
 * Where an element's name ends in `text`, for the element starting at `start`: after its `<` and
 * its name, and after a text type that follows the name, as `:markdown` in `<Note:markdown`.
 */
function nameEnd(text: string, node: SourceNode, start: number): number {
  let end = start + 1 + (node.name ?? "").length;
  if (text[end] === ":") {
    end += 1;
    while (end < text.length && /[A-Za-z0-9_.-]/.test(text[end]!)) {
      end += 1;
    }
  }
  return end;
}

/** The name the type of text shown to people goes by. An alias for now; it may become built in. */
const UI_TEXT = "uitext";

/**
 * The nodes that are text shown to people: the string literals and element text that a property,
 * a field, a parameter or a declaration typed `uitext` holds, where `uitext` is the type itself
 * or any alias that leads to it. Which strings these are comes from the declarations, never from
 * how a string looks.
 */
export function uiTextNodes(index: TreeIndex): Set<number> {
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
 * The string literals an attribute holds directly, with the format its property's visualizer gives
 * them: from `<PropertyVisualizer property=id format=id />`, or from the visualizer of the alias its type
 * names.
 */
export function formattedNodes(index: TreeIndex): Map<number, SourceValueFormat> {
  const { tree } = index;
  const found = new Map<number, SourceValueFormat>();
  tree.nodes.forEach((node, at) => {
    if (node.role !== "literal" || !(node.value ?? "").startsWith('"') || node.parent === undefined) {
      return;
    }
    const attribute = tree.nodes[node.parent]!;
    const element = attribute.parent === undefined ? undefined : tree.nodes[attribute.parent];
    if (attribute.role !== "attribute" || element?.declaration === undefined) {
      return;
    }
    const visualizer = tree.declarations[element.declaration]?.visualizer;
    const format = visualizer?.properties?.find((property) => property.name === attribute.name)?.format;
    if (format !== undefined) {
      found.set(at, format);
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

/**
 * A string literal's content: its quotes taken off, and `\"` and `\\` read as a quote and a
 * backslash. Nothing when it holds any other escape, as `\n`: the source edit writes back only those
 * two, so editing it would lose the others.
 */
export function decodeString(written: string): string | undefined {
  const inner = written.length >= 2 && written.startsWith('"') && written.endsWith('"') ? written.slice(1, -1) : written;
  if (inner.replace(/\\[\\"]/g, "").includes("\\")) {
    return undefined;
  }
  return inner.replace(/\\([\\"])/g, "$1");
}
