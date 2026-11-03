/**
 * The source reading: the document's text with each node of the source tree wrapped in a span that
 * carries its key. Code reads as written; an element reads as its name and properties on one line
 * when they fit there, and as a group, a row to each property, when they do not or when it holds
 * items, without tags or quotes, with its id moved to where its visualizer puts it and its booleans
 * read as marks. Colors come from the host, as ranges over the
 * text, so a host colors the reading with the highlighter and themes it colors its editor with.
 */
import type { SourceDeclarationKind, SourceNode, SourceValueFormat } from "@nx-lang/language-protocol";

import { isComposition } from "./compact.js";
import { isElementFunction, VALUE_ROLES } from "./card.js";
import { isBooleanType, presentationOf, tooltipOf, type Presentation } from "./labels.js";
import { attributeElement, elementProperties, type TreeIndex } from "./tree.js";

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

/** The Lucide pin a row's pin button draws. */
const PIN_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/></svg>';

/** How many properties a row's summary shows when its visualizer names none. */
const DEFAULT_SUMMARY_LENGTH = 3;

/** What the reading draws with, beyond the text and its colors. */
export interface SourceLayout {
  /** The keys of the composition elements whose inspectors are open. */
  readonly details?: ReadonlySet<string> | undefined;
  /** The keys of the composition elements whose inspectors are pinned open. */
  readonly pinned?: readonly string[] | undefined;
  /** The keys of the element values unfolded into nested groups of fields in an inspector. */
  readonly valuesOpen?: ReadonlySet<string> | undefined;
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
  // A string literal's words from `start` to `end`, without its quotes, written in the color the text
  // has there, with its escapes read; or nothing, when it has none to read, so it is written as it
  // is. Only where the reading draws no quotes, or quiet ones of its own: in a string drawn as code,
  // between its own quotes, a quote read from `\"` would look like the string's end.
  const writeReadable = (parent: HTMLElement, start: number, end: number): boolean => {
    const written = text.slice(start + 1, end - 1);
    const words = readable(start, end);
    if (words === written) {
      return false;
    }
    const range = colors.find((candidate) => candidate.start <= start + 1 && candidate.end > start + 1);
    piece(parent, start + 1, end - 1, range, words);
    return true;
  };

  const uiText = uiTextNodes(index);
  const formats = formattedNodes(index);
  const identifiers = new Set([...formats].flatMap(([at, format]) => (format === "id" ? [at] : [])));

  // A small square of the color a string names, before it, for a string its visualizer formats as a
  // color. Nothing when the string names no color: the square would show a color that is not there.
  const swatch = (at: number, start: number, end: number): HTMLElement | undefined => {
    const raw = text.slice(start, end);
    const native = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(raw);
    const known = tree.nodes[at]?.color;
    if (!native && known === undefined && formats.get(at) !== "color") {
      return undefined;
    }
    const square = document.createElement("span");
    square.className = "swatch";
    // A color, never anything else: `background-color` takes no URL.
    square.style.backgroundColor = known ?? (native ? raw : text.slice(start + 1, end - 1).trim());
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
        const square = quoted || node.color !== undefined || text[start] === "#" ? swatch(child, start, end) : undefined;
        if (square !== undefined) {
          element.append(square);
        }
        writeNodes(element, index.children[child] ?? [], start, end, idPlans.get(child));
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
    // A component's `State.Update` is an action too, so it reads with an action's icon.
    const kind = declaration?.kind === "record" && declaration.name.endsWith(".State.Update") ? "action" : declaration?.kind;
    const drawn = draw(declaration?.visualizer?.icon) ?? draw(kind === undefined ? undefined : KIND_ICONS[kind]);
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
  const rows = new Map<number, boolean>();
  /** Whether an element in the composition reads as a row: its tag holds no comment a row would lose. */
  const rowAt = (at: number): boolean => {
    let row = rows.get(at);
    if (row === undefined) {
      row =
        isComposition(index, at) &&
        childrenOf(at).every((child) => {
          const node = tree.nodes[child]!;
          if (isNote(child)) {
            return true;
          }
          return node.role === "attribute" && (isContent(child) || !childrenOf(child).some(isNote));
        });
      rows.set(at, row);
    }
    return row;
  };

  const markFit = (element: HTMLElement, at: number): void => {
    if (rowAt(at)) {
      return;
    }
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
      shape = rowAt(at) ? "group" : lineable(at) ? "line" : sheetable(at) ? "group" : "written";
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
  const inHeader = (element: number, attribute: number): boolean => !rowAt(element) && !isWrapped(element) && inlineValue(attribute);

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
    if (value.role === "literal" && /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(written)) {
      const words = marked(at, "span", "colored");
      const square = swatch(at, start, end);
      if (square !== undefined) words.append(square);
      writeText(words, start, end);
      cell.append(words);
      return;
    }
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
        if (!writeReadable(words, start, end)) {
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
  const foldToggle = (key: string, folded: boolean, button = false): HTMLElement => {
    const toggle = document.createElement(button ? "button" : "span");
    if (button) {
      toggle.setAttribute("type", "button");
      toggle.tabIndex = -1;
    }
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

  // ---- Rows and inspectors -------------------------------------------------------------------

  /** A copy of a part of the reading, to show again: it marks no node, so each node is drawn once. */
  const unmarked = (element: HTMLElement): HTMLElement => {
    for (const marked of [element, ...element.querySelectorAll<HTMLElement>("[data-key]")]) {
      delete marked.dataset["key"];
      delete marked.dataset["role"];
      marked.classList.remove("n", "fit-check");
      marked.removeAttribute("id");
    }
    return element;
  };

  /** The presentation of each property an element's row and inspector name. */
  const presentations = (at: number): ((name: string) => Presentation) => {
    const node = tree.nodes[at]!;
    const visualizer = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.visualizer;
    const declared = elementProperties(tree, node) ?? [];
    const found = new Map<string, Presentation>();
    return (name) => {
      let presentation = found.get(name);
      if (presentation === undefined) {
        const property = declared.find((candidate) => candidate.name === name);
        presentation = presentationOf(visualizer, name, isBooleanType(tree, property?.type));
        found.set(name, presentation);
      }
      return presentation;
    };
  };

  /**
   * The lead of a property's field or summary item. A field's is its label, after its icon when it
   * has one. A summary item's is its icon, else its short label, else its label as a caption. Either
   * carries the property's label and its name as source writes it as a tooltip.
   */
  const leadOf = (presentation: Presentation, place: "field" | "summary"): HTMLElement => {
    const lead = document.createElement("span");
    lead.title = tooltipOf(presentation);
    const drawn = presentation.icon === undefined ? undefined : layout.icons?.(presentation.icon);
    if (place === "field") {
      lead.className = "field-lead lead-caption";
      if (drawn !== undefined) {
        const glyph = document.createElement("span");
        glyph.className = "lead-glyph";
        glyph.setAttribute("aria-hidden", "true");
        glyph.append(drawn.cloneNode(true));
        lead.append(glyph);
      }
      lead.append(presentation.label);
      return lead;
    }
    if (drawn !== undefined) {
      lead.className = `${place}-lead lead-icon`;
      lead.append(drawn.cloneNode(true));
      lead.setAttribute("aria-label", presentation.label);
    } else if (presentation.short !== undefined) {
      lead.className = `${place}-lead lead-short`;
      lead.textContent = presentation.short;
      lead.setAttribute("aria-label", presentation.label);
    } else {
      lead.className = `${place}-lead lead-caption`;
      lead.textContent = presentation.label;
    }
    return lead;
  };

  /** The one value an attribute holds, or nothing when it holds several or a comment. */
  const soleValue = (attribute: number): number | undefined => {
    const values = childrenOf(attribute);
    return values.length === 1 && !isNote(values[0]!) ? values[0] : undefined;
  };

  /** Whether an attribute's value is the boolean `true` or `false`, and which. */
  const booleanOf = (attribute: number): boolean | undefined => {
    const value = soleValue(attribute);
    const written = value === undefined ? undefined : tree.nodes[value]!;
    return written?.role === "literal" && (written.value === "true" || written.value === "false") ? written.value === "true" : undefined;
  };

  /**
   * The attributes a row's summary shows: those its visualizer's summary names that the element
   * writes, in that order, or else the first few it writes whose values are words or read on one
   * line, its title and id left out. A false boolean is left out either way.
   */
  /**
   * The content of `at` that reads inline, as a property does: text and embedded values, as
   * `<Text>NX graphics</Text>` holds, with no element or block in it. However long, its row shows
   * its first words.
   */
  const inlineContent = (at: number): number | undefined => {
    const content = childrenOf(at).filter((child) => tree.nodes[child]!.role === "attribute" && isContent(child));
    if (content.length !== 1) {
      return undefined;
    }
    const child = content[0]!;
    const items = childrenOf(child);
    const blocks = new Set(["element", "condition", "match", "matchArm", "loop", "sequence", "binding"]);
    if (items.length === 0 || items.some((item) => blocks.has(tree.nodes[item]!.role))) {
      return undefined;
    }
    const [from, to] = inlineRange(child);
    return from < to ? child : undefined;
  };

  /** Whether an attribute holds words: content of text, or a quoted string. */
  const isWords = (attribute: number): boolean => {
    if (isContent(attribute)) {
      return true;
    }
    const value = soleValue(attribute);
    return value !== undefined && tree.nodes[value]!.role === "literal" && (tree.nodes[value]!.value ?? "").startsWith('"');
  };

  /** Where content's items run, blank space they hold included; it reads collapsed, as in prose. */
  const itemsRange = (content: number): [number, number] => {
    const items = childrenOf(content);
    // An embedded value's closing brace is the content's, past its last item.
    // So is an embedded value's opening brace, before its first.
    return [Math.min(startOf(tree.nodes[items[0]!]!), startOf(tree.nodes[content]!)), Math.max(endOf(tree.nodes[items[items.length - 1]!]!), endOf(tree.nodes[content]!))];
  };

  /** Where inline content's text runs, without the blank space around it. */
  const inlineRange = (content: number): [number, number] => {
    const items = childrenOf(content);
    let from = Math.min(startOf(tree.nodes[items[0]!]!), startOf(tree.nodes[content]!));
    let to = Math.max(endOf(tree.nodes[items[items.length - 1]!]!), endOf(tree.nodes[content]!));
    while (from < to && /\s/.test(text[from]!)) {
      from++;
    }
    while (to > from && /\s/.test(text[to - 1]!)) {
      to--;
    }
    return [from, to];
  };

  const summaryOf = (at: number, title: number | undefined): number[] => {
    const node = tree.nodes[at]!;
    const visualizer = node.declaration === undefined ? undefined : tree.declarations[node.declaration]?.visualizer;
    const inline = inlineContent(at);
    const written = childrenOf(at).filter((child) => tree.nodes[child]!.role === "attribute" && (!isContent(child) || child === inline));
    const shows = (attribute: number): boolean => attribute === inline || (booleanOf(attribute) !== false && soleValue(attribute) !== undefined);
    if (visualizer?.summary !== undefined && visualizer.summary.length > 0) {
      const named = visualizer.summary.flatMap((name) => {
        const found = written.find((child) => tree.nodes[child]!.name === name);
        return found !== undefined && shows(found) ? [found] : [];
      });
      // Inline content reads first even where the summary leaves it out, since nothing else shows it.
      return inline !== undefined && !named.includes(inline) ? [inline, ...named] : named;
    }
    if (inline !== undefined) {
      return [inline, ...written.filter((child) => child !== inline && child !== title && child !== idPlans.get(at)?.attribute && shows(child) && soleValue(child) !== undefined && tree.nodes[soleValue(child)!]!.role !== "element" && (isWords(child) || onOneLine(startOf(tree.nodes[soleValue(child)!]!), endOf(tree.nodes[soleValue(child)!]!)))).slice(0, DEFAULT_SUMMARY_LENGTH - 1)];
    }
    return written
      .filter((child) => {
        const value = soleValue(child);
        return (
          child !== title &&
          child !== idPlans.get(at)?.attribute &&
          shows(child) &&
          value !== undefined &&
          tree.nodes[value]!.role !== "element" &&
          (isWords(child) || onOneLine(startOf(tree.nodes[value]!), endOf(tree.nodes[value]!)))
        );
      })
      .slice(0, DEFAULT_SUMMARY_LENGTH);
  };

  /** The one attribute `at` writes, when it writes just one, on one line and not an element. */
  const soleSetting = (at: number): number | undefined => {
    const written = childrenOf(at).filter((child) => tree.nodes[child]!.role === "attribute" && !isContent(child));
    if (written.length !== 1) {
      return undefined;
    }
    const value = soleValue(written[0]!);
    if (value === undefined || tree.nodes[value]!.role === "element" || !onOneLine(startOf(tree.nodes[value]!), endOf(tree.nodes[value]!))) {
      return undefined;
    }
    return written[0];
  };

  /** One item of a row's summary: the property's lead and a copy of its value, as its field reads. */
  const summaryItem = (attribute: number, presentation: Presentation): HTMLElement => {
    const item = document.createElement("span");
    item.className = "summary-item";
    item.title = tooltipOf(presentation);
    hoverable(item, attribute);
    if (isWords(attribute)) {
      // Words read as their first few, cut and faded where they run long.
      item.classList.add("summary-words");
    }
    if (isContent(attribute)) {
      // Inline content reads as its text, as the element's title does, with no label.
      const words = document.createElement("span");
      words.className = "summary-value summary-text";
      writeNodes(words, childrenOf(attribute), ...itemsRange(attribute));
      item.append(unmarked(words));
      return item;
    }
    const lead = leadOf(presentation, "summary");
    const truth = booleanOf(attribute);
    if (truth === true) {
      // A true boolean reads as what it turns on: its icon and its label.
      item.classList.add("summary-flag");
      if (lead.classList.contains("lead-caption")) {
        item.append(lead);
      } else {
        const label = document.createElement("span");
        label.className = "summary-label";
        label.textContent = presentation.label;
        item.append(lead, label);
      }
      return item;
    }
    const value = document.createElement("span");
    value.className = "summary-value";
    const at = soleValue(attribute);
    const only = at !== undefined && tree.nodes[at]!.role === "element" ? soleSetting(at) : undefined;
    if (only !== undefined) {
      // An element value that sets one thing, as a paint its color, reads as that value.
      writeValue(value, only);
    } else if (at !== undefined && tree.nodes[at]!.role === "element") {
      // Any other element value reads by its kind, as its field does when folded.
      value.textContent = tree.nodes[at]!.name ?? "";
    } else {
      writeValue(value, attribute);
      if (at !== undefined && tree.nodes[at]!.role === "reference") {
        value.classList.add("reference-token");
      }
    }
    item.append(lead, unmarked(value));
    return item;
  };

  /** A field of an inspector for an attribute the element writes, its value editable in place. */
  const writtenField = (attribute: number, presentation: Presentation): HTMLElement => {
    const field = marked(attribute, "span", "sheet-row prop-field");
    const lead = leadOf(presentation, "field");
    lead.classList.add("sheet-name");
    hoverable(lead, attribute);
    const cell = document.createElement("span");
    cell.className = "sheet-value";
    cell.title = tooltipOf(presentation);
    field.append(lead, cell);
    if (booleanOf(attribute) !== undefined) {
      // A checkbox beside its label.
      field.classList.add("boolean-field");
      writeValue(cell, attribute);
      return field;
    }
    const nested = soleValue(attribute);
    if (nested !== undefined && tree.nodes[nested]!.role === "element" && nestable(nested)) {
      writeNested(field, cell, nested);
      return field;
    }
    if (spansLines(attribute)) {
      field.classList.add("field-below");
      const below = document.createElement("span");
      below.className = "sheet-below";
      writeValue(below, attribute);
      field.append(below);
    } else {
      const value = soleValue(attribute);
      writeValue(cell, attribute);
      if (value !== undefined && tree.nodes[value]!.role === "reference") {
        cell.classList.add("reference-token");
      }
    }
    return field;
  };

  /**
   * Whether an element value reads as a nested group of fields: its tag holds properties alone, with
   * no comment a field would lose and nothing it holds.
   */
  const nestable = (at: number): boolean =>
    childrenOf(at).every((child) => {
      const node = tree.nodes[child]!;
      return node.role === "attribute" && !isContent(child) && !childrenOf(child).some(isNote);
    });

  /**
   * An element value in its property's field: its icon, kind and summary, which unfold into a
   * nested group of its own fields, edited as any others.
   */
  const writeNested = (field: HTMLElement, cell: HTMLElement, at: number): void => {
    const node = tree.nodes[at]!;
    const start = startOf(node);
    const plan = idPlans.get(at);
    if (plan !== undefined) {
      plan.written = true;
    }
    const open = layout.valuesOpen?.has(node.key) === true;
    field.classList.add("nested-field");
    field.classList.toggle("open", open);
    const head = marked(at, "span", "nested-head");
    const toggle = document.createElement("button");
    toggle.type = "button";
    toggle.tabIndex = -1;
    toggle.className = open ? "fold value-fold" : "fold value-fold folded";
    toggle.dataset["valueKey"] = node.key;
    toggle.setAttribute("aria-expanded", String(open));
    toggle.setAttribute("aria-label", `${open ? "Collapse" : "Expand"} ${node.name ?? "value"}`);
    head.append(toggle);
    const marker = elementIcon(at);
    if (marker !== undefined) {
      head.append(marker);
    }
    const kind = document.createElement("span");
    kind.className = "nested-kind";
    hoverable(kind, at);
    writeText(kind, start + 1, nameEnd(text, node, start));
    head.append(kind);
    if (!open) {
      const summary = document.createElement("span");
      summary.className = "row-summary nested-summary";
      const present = presentations(at);
      for (const attribute of summaryOf(at, titleOf(at))) {
        summary.append(summaryItem(attribute, present(tree.nodes[attribute]!.name ?? "")));
      }
      if (summary.childElementCount > 0) {
        const more = document.createElement("span");
        more.className = "summary-more";
        more.hidden = true;
        summary.append(more);
        head.append(summary);
      }
    }
    cell.append(head);
    if (open) {
      const declaration = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
      const inspector = writeInspector(at, presentations(at), declaration?.visualizer?.properties?.map((property) => property.name) ?? []);
      inspector.classList.add("nested-inspector");
      // The nested element's own node holds its fields, so selecting inside it selects in it.
      head.append(inspector);
    }
  };

  /**
   * The empty field of a property the element does not write: dashed, with its lead, and a value
   * cell the element fills with the property's default and its editor.
   */
  const unwrittenField = (key: string, presentation: Presentation, boolean: boolean): HTMLElement => {
    const field = document.createElement("span");
    field.className = "sheet-row prop-field more dimmed-row";
    field.dataset["moreOf"] = key;
    field.dataset["name"] = presentation.name;
    field.title = tooltipOf(presentation);
    const lead = leadOf(presentation, "field");
    lead.classList.add("sheet-name");
    const cell = document.createElement("span");
    cell.className = "sheet-value";
    if (boolean) {
      field.classList.add("boolean-field");
    }
    field.append(lead, cell);
    return field;
  };

  /**
   * An element of the composition as one row: its children's chevron when it holds content, its
   * icon, kind, id and title, its summary, cut by the element to what fits, and its pin. Under it,
   * while open, its inspector: a field to each property it declares, written or not;
   * then, unless folded, what it holds.
   */
  const writeRow = (at: number, start: number): HTMLElement => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const folded = layout.folded?.has(node.key) === true;
    const open = layout.details?.has(node.key) === true;
    const pinned = layout.pinned?.includes(node.key) === true;
    const declaration = node.declaration === undefined ? undefined : tree.declarations[node.declaration];
    const visualizer = declaration?.visualizer;
    const present = presentations(at);
    const children = childrenOf(at);
    const inline = inlineContent(at);
    const content = children.filter((child) => tree.nodes[child]!.role === "attribute" && isContent(child) && child !== inline);
    const hasContent = content.some((child) => childrenOf(child).length > 0);
    const row = marked(at, "span", "sheet row-element");
    row.classList.toggle("folded", folded && hasContent);
    row.classList.toggle("open", open);
    row.classList.toggle("pinned", pinned);
    row.style.setProperty("--indent", String(start - (text.lastIndexOf("\n", start - 1) + 1)));
    row.setAttribute("role", "treeitem");
    row.setAttribute("aria-selected", "false");
    row.setAttribute("aria-label", node.name ?? "element");
    if (hasContent) {
      row.setAttribute("aria-expanded", String(!folded));
    }
    if (plan !== undefined) {
      plan.written = true;
    }

    const head = document.createElement("span");
    head.className = "sheet-head row-head";
    if (hasContent) {
      const toggle = foldToggle(node.key, folded, true);
      toggle.setAttribute("aria-label", `${folded ? "Expand" : "Collapse"} children of ${node.name}`);
      head.append(toggle);
    }
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
      head.append(unmarked(idChip(plan)));
    }
    const title = titleOf(at);
    if (title !== undefined) {
      const words = document.createElement("span");
      words.className = "row-title";
      hoverable(words, title);
      writeValue(words, title);
      head.append(unmarked(words));
    }
    if (plan !== undefined && plan.atEnd) {
      head.append(unmarked(idChip(plan)));
    }
    const summary = document.createElement("span");
    summary.className = "row-summary";
    summary.hidden = open;
    for (const attribute of summaryOf(at, title)) {
      summary.append(summaryItem(attribute, present(tree.nodes[attribute]!.name ?? "")));
    }
    if (summary.childElementCount > 0) {
      const more = document.createElement("span");
      more.className = "summary-more";
      more.hidden = true;
      summary.append(more);
      head.append(summary);
    }
    const pin = document.createElement("button");
    pin.type = "button";
    pin.tabIndex = -1;
    pin.className = "row-pin";
    pin.dataset["detailsKey"] = node.key;
    pin.dataset["detailsAction"] = "pin";
    pin.title = pinned ? "Unpin" : "Pin open";
    pin.setAttribute("aria-label", `${pinned ? "Unpin" : "Pin"} ${node.name ?? "element"}`);
    pin.setAttribute("aria-pressed", String(pinned));
    pin.innerHTML = PIN_ICON;
    head.append(pin);
    row.append(head);

    if (open) {
      const inspector = writeInspector(at, present, visualizer?.properties?.map((property) => property.name) ?? []);
      if (inline !== undefined) {
        // Inline content edits in the inspector, first, as the property it is.
        const presentation = present(tree.nodes[inline]!.name ?? "");
        const field = marked(inline, "span", "sheet-row prop-field content-field");
        const lead = leadOf(presentation, "field");
        lead.classList.add("sheet-name");
        hoverable(lead, inline);
        const cell = document.createElement("span");
        cell.className = "sheet-value held-text";
        cell.title = tooltipOf(presentation);
        writeNodes(cell, childrenOf(inline), ...itemsRange(inline));
        field.append(lead, cell);
        inspector.querySelector(":scope > .inspector-fields")!.prepend(field);
      }
      row.append(inspector);
    }
    if (!folded) {
      for (const child of content) {
        const held = marked(child, "span", "sheet-held");
        const items = childrenOf(child);
        if (items.some((item) => tree.nodes[item]!.role === "text") && onOneLine(startOf(tree.nodes[child]!), endOf(tree.nodes[child]!))) {
          // A line of text reads as written, its embedded values in their braces.
          held.classList.add("held-text");
          writeNodes(held, items, startOf(tree.nodes[child]!), endOf(tree.nodes[child]!));
        } else {
          writeItems(held, items);
        }
        row.append(held);
      }
    }
    for (const child of children) {
      if (isNote(child)) {
        const line = document.createElement("span");
        line.className = "sheet-note";
        writeNodes(line, [child], startOf(tree.nodes[child]!), endOf(tree.nodes[child]!));
        row.append(line);
      }
    }
    return row;
  };

  /**
   * An element's inspector: a field to each property its type declares, in the order its
   * visualizer lists them, then the rest in declaration order, then any it writes that its type
   * does not declare, each its label beside its value.
   */
  const writeInspector = (at: number, present: (name: string) => Presentation, order: readonly string[]): HTMLElement => {
    const node = tree.nodes[at]!;
    const kindName = node.name ?? "";
    const declared = (elementProperties(tree, node) ?? []).filter((property) => property.flags?.includes("content") !== true);
    // A property written twice, as an error leaves it, reads its first in its place, and the rest
    // after the element's properties.
    const written = new Map<string, number>();
    const repeated: number[] = [];
    for (const child of childrenOf(at)) {
      if (tree.nodes[child]!.role === "attribute" && !isContent(child)) {
        const name = tree.nodes[child]!.name ?? "";
        if (written.has(name)) {
          repeated.push(child);
        } else {
          written.set(name, child);
        }
      }
    }
    const rank = (name: string): number => {
      const found = order.indexOf(name);
      return found < 0 ? order.length : found;
    };
    const names = declared
      .map((property, position) => ({ name: property.name, position }))
      .sort((a, b) => rank(a.name) - rank(b.name) || a.position - b.position)
      .map(({ name }) => name);
    for (const name of written.keys()) {
      if (!names.includes(name)) {
        names.push(name);
      }
    }
    const inspector = document.createElement("span");
    inspector.className = "inspector";
    inspector.setAttribute("role", "group");
    inspector.setAttribute("aria-label", `Properties of ${kindName}`);
    const fields = document.createElement("span");
    fields.className = "inspector-fields";
    for (const name of names) {
      const attribute = written.get(name);
      const property = declared.find((candidate) => candidate.name === name);
      fields.append(
        attribute !== undefined
          ? writtenField(attribute, present(name))
          : unwrittenField(node.key, present(name), isBooleanType(tree, property?.type))
      );
    }
    for (const attribute of repeated) {
      fields.append(writtenField(attribute, present(tree.nodes[attribute]!.name ?? "")));
    }
    inspector.append(fields);
    return inspector;
  };

  const writeSheet = (at: number, start: number, end: number): HTMLElement => {
    const node = tree.nodes[at]!;
    const plan = idPlans.get(at);
    const folded = layout.folded?.has(node.key) === true;
    if (rowAt(at)) {
      return writeRow(at, start);
    }
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
      const owner = attributeElement(tree, place);
      spelled =
        owner === undefined
          ? undefined
          : elementProperties(tree, tree.nodes[owner]!)?.find((property) => property.name === holder.name)?.type;
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
        // Measured from the item's start. An item that starts after other text on its line, as text
        // on its element's tag does, is measured from its least indented later line when one sits
        // further left, so no line is drawn before the item.
        const later = startsItsLine(start)
          ? []
          : text
              .slice(start, end)
              .split("\n")
              .slice(1)
              .filter((written) => written.trim() !== "")
              .map((written) => written.length - written.trimStart().length);
        const column = Math.min(start - (text.lastIndexOf("\n", start - 1) + 1), ...later);
        line.style.setProperty("--column", String(column));
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
  // The braces of an `if` or a `for` read quieter than the code they hold.
  for (const token of root.querySelectorAll<HTMLElement>(':is(.n[data-role="condition"], .n[data-role="loop"]) > .t')) {
    if (/^\s*[{}]\s*$/.test(token.textContent ?? "")) {
      token.classList.add("block-brace");
    }
  }
  if (root.querySelector(".row-element") !== null) {
    root.setAttribute("role", "tree");
    root.setAttribute("aria-label", "Composition");
  }
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
 * Whether a type, as spelled, is `uitext` itself or an alias that leads to it, among the tree's
 * declarations.
 */
export function uiTextType(tree: TreeIndex["tree"]): (spelled: string | undefined) => boolean {
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
    const alias = tree.declarations.find((entry) => entry.kind === "alias" && entry.name === name);
    const result = alias !== undefined && isUIText(alias.type, depth + 1);
    known.set(name, result);
    return result;
  };
  return (spelled) => isUIText(spelled);
}

/**
 * The nodes that are text shown to people: the string literals and element text that a property,
 * a field, a parameter or a declaration typed `uitext` holds, where `uitext` is the type itself
 * or any alias that leads to it. Which strings these are comes from the declarations, never from
 * how a string looks.
 */
export function uiTextNodes(index: TreeIndex): Set<number> {
  const { tree } = index;
  const isUIText = uiTextType(tree);

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
      const element = attributeElement(tree, at);
      const property =
        element === undefined
          ? undefined
          : elementProperties(tree, tree.nodes[element]!)?.find((candidate) => candidate.name === node.name);
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
    const elementAt = attribute.role === "attribute" ? attributeElement(tree, node.parent) : undefined;
    const element = elementAt === undefined ? undefined : tree.nodes[elementAt];
    if (element?.declaration === undefined) {
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
