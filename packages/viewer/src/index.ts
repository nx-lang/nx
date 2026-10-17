/**
 * `<nx-viewer>`: an NX document's source tree as a reading view.
 *
 * <para>The element takes a `SourceTree`, as the language service's `sourceTree` query answers it,
 * and the text of the document it describes, and shows that text as it reads rather than as it is
 * typed: an element written over several lines reads as a group under its name, with a row to each
 * property and no tags; an element on one line reads as its name and properties; and the rest
 * reads as written, in the colors the host's editor gives it. Every piece carries its node's key.
 * It has no framework dependency and keeps its markup and styles in a shadow root.</para>
 */
import type { EditorRange, SourceRole, SourceTree } from "@nx-lang/language-protocol";

import { defaultFolded, readingRuns, renderSource, type NxViewerColor, type NxViewerIcons, type ReadingRun } from "./source.js";
import { styles } from "./styles.js";
import { indexTree, nodeOf, type TreeIndex } from "./tree.js";

export { nodeAtSpan } from "./tree.js";
export { keyAtRange, rangeOfKey } from "./selection.js";
export type { NxViewerColor, NxViewerIcons } from "./source.js";

/** The tag the element is defined under. */
export const NX_VIEWER_TAG = "nx-viewer";

/** The event fired when a reader selects a node. */
export const NX_SELECT_EVENT = "nx-select";

/** The event fired when a reader folds or unfolds a group or a declaration. */
export const NX_FOLD_EVENT = "nx-fold";

/** What the `nx-fold` event carries. */
export interface NxFoldDetail {
  /** The keys of every unit folded now. */
  readonly folded: readonly string[];
  /**
   * The selection after the fold: the folded unit that holds it when a fold hid it, or nothing
   * when the unit that hid it is a run, which is not a node.
   */
  readonly selection: string | undefined;
}

/** What the `nx-select` event carries. */
export interface NxSelectDetail {
  /** The selected node's key. */
  readonly key: string;
  /** Its role. */
  readonly role: SourceRole;
  /** The range it covers. */
  readonly range: EditorRange;
}

/** The element's properties, which a host may set before the element is defined. */
const PROPERTIES = ["tree", "text", "colors", "icons", "folded", "selection", "stale"] as const;

// A module imported where there is no DOM still loads; the element is only defined where
// `HTMLElement` exists.
const ElementBase = (typeof HTMLElement === "undefined" ? class {} : HTMLElement) as typeof HTMLElement;

/**
 * What a click selects, and what the pointer and the arrow keys move between: an element on lines
 * of its own (through its header), a row of a group, an element on one line, an element written as
 * its tags that starts a line, or a block written as it is, as an `if` or a `for`. A group's header
 * stands for its group.
 */
const UNITS = ".sheet-head, .sheet-row, .line-element, .tag-line, .block-line";

/** The units as they are drawn selected: a group stands for itself, not through its header. */
const DRAWN_UNITS = ".sheet, .sheet-row, .line-element, .tag-line, .block-line";

/**
 * What `element` reads as, word by word: its text, with an icon read by its label, for telling a
 * screen reader what is selected.
 */
function spoken(element: Element): string {
  const words: string[] = [];
  const walk = (node: Node) => {
    if (node.nodeType === 3) {
      words.push(node.textContent ?? "");
      return;
    }
    if (node.nodeType !== 1) {
      return;
    }
    const child = node as Element;
    const label = child.getAttribute("role") === "img" ? child.getAttribute("aria-label") : null;
    if (label !== null) {
      words.push(`${label} `);
      return;
    }
    child.childNodes.forEach(walk);
  };
  walk(element);
  return words.join("").replace(/\s+/g, " ").trim();
}

function keyOf(element: Element): string | undefined {
  return (element as HTMLElement).dataset["key"];
}

/**
 * The `<nx-viewer>` element.
 *
 * <para>The attribute `stale` mirrors the property of the same name. Everything else is a property:
 * `tree`, `text`, `colors`, `icons`, `folded` and `selection`.</para>
 */
export class NxViewerElement extends ElementBase {
  static observedAttributes = ["stale"];

  #tree: SourceTree | undefined;
  #text = "";
  #colors: readonly NxViewerColor[] = [];
  #icons: NxViewerIcons | undefined;
  /** What the host or the reader folded; nothing until either does, when the defaults apply. */
  #chosenFolded: Set<string> | undefined;
  /** What is folded now: the chosen keys, or the defaults for this tree. */
  #folded = new Set<string>();
  /** The fold just toggled and how it was, so its chevron can turn from there. */
  #turning: { key: string; wasFolded: boolean } | undefined;
  /** The runs of supporting declarations of the tree shown. */
  #runs: ReadingRun[] = [];
  #index: TreeIndex | undefined;
  #selection: string | undefined;
  /** Each key's rendering. */
  #rendered = new Map<string, HTMLElement>();
  /** The unit under the pointer, highlighted as a row. */
  #hoveredUnit: HTMLElement | undefined;
  /** The body's width the reading was last measured at, for what fits on a line. */
  #fitWidth = -1;
  /** The measure waiting for the pane's width to settle. */
  #remeasureTimer: ReturnType<typeof setTimeout> | undefined;

  readonly #badge: HTMLSpanElement;
  readonly #body: HTMLDivElement;
  /** The highlights of the selected and the hovered unit, drawn behind the text. */
  readonly #highlights: HTMLDivElement;
  /** Says what the arrow keys selected, for a screen reader, since focus stays on the body. */
  readonly #announcer: HTMLDivElement;

  constructor() {
    super();
    const shadow = this.attachShadow({ mode: "open" });
    const document = this.ownerDocument;

    const style = document.createElement("style");
    style.textContent = styles;

    const frame = document.createElement("div");
    frame.className = "frame";

    const toolbar = document.createElement("div");
    toolbar.className = "toolbar";
    this.#badge = document.createElement("span");
    this.#badge.className = "badge";
    this.#badge.textContent = "Out of date";
    this.#badge.hidden = true;
    toolbar.append(this.#badge);

    this.#body = document.createElement("div");
    this.#body.className = "body";
    // The body takes focus so the arrow keys can walk the selection.
    this.#body.tabIndex = 0;
    this.#body.setAttribute("role", "region");
    this.#body.setAttribute(
      "aria-label",
      "Reading view; the up and down arrows select the next row, left its group and right what it holds"
    );

    this.#highlights = document.createElement("div");
    this.#highlights.className = "highlights";
    this.#highlights.setAttribute("aria-hidden", "true");

    this.#announcer = document.createElement("div");
    this.#announcer.className = "announcer";
    this.#announcer.setAttribute("role", "status");
    this.#announcer.setAttribute("aria-live", "polite");

    frame.append(toolbar, this.#body, this.#announcer);
    shadow.append(style, frame);

    this.#body.addEventListener("click", (event) => this.#onClick(event));
    // A focus ring is for a reader on the keyboard; after a click, the selection says where they are.
    this.#body.addEventListener("pointerdown", () => this.#body.classList.add("by-pointer"));
    frame.addEventListener("keydown", () => this.#body.classList.remove("by-pointer"));
    frame.addEventListener("keydown", (event) => this.#onKeyDown(event));
    this.#body.addEventListener("pointerover", (event) => this.#setHoveredUnit(this.#unitOf(event.target as Element)));
    this.#body.addEventListener("pointerleave", () => this.#setHoveredUnit(undefined));
    // The highlights follow the layout: a wider pane moves the right edge they run to.
    if (typeof ResizeObserver !== "undefined") {
      new ResizeObserver(() => {
        this.#paintHighlights();
        if (this.#body.clientWidth === this.#fitWidth) {
          return;
        }
        // What fits on a line changes with the pane's width. A reading first given a width is
        // measured at once; one already measured, once the width settles, so dragging a divider
        // does not draw it again on every frame.
        clearTimeout(this.#remeasureTimer);
        if (this.#fitWidth <= 0) {
          this.#remeasure();
        } else {
          this.#remeasureTimer = setTimeout(() => this.#body.clientWidth !== this.#fitWidth && this.#remeasure(), 120);
        }
      }).observe(this.#body);
    }

    // A property set before the element was defined hides the accessor; set it again through it.
    for (const name of PROPERTIES) {
      if (Object.prototype.hasOwnProperty.call(this, name)) {
        const value = (this as Record<string, unknown>)[name];
        delete (this as Record<string, unknown>)[name];
        (this as Record<string, unknown>)[name] = value;
      }
    }
  }

  /** The source tree to show, as the language service's `sourceTree` query answers it. */
  get tree(): SourceTree | undefined {
    return this.#tree;
  }

  set tree(tree: SourceTree | undefined) {
    this.#tree = tree;
    this.#render();
  }

  /** The text of the document the tree describes. */
  get text(): string {
    return this.#text;
  }

  set text(text: string) {
    this.#text = text ?? "";
    this.#render();
  }

  /**
   * The colors of the text, as ranges over `text` in UTF-16 offsets, in text order. A host supplies
   * them from the highlighter and themes it colors its editor with, so the two agree. Without them
   * the text reads in the element's foreground color.
   */
  get colors(): readonly NxViewerColor[] {
    return this.#colors;
  }

  set colors(colors: readonly NxViewerColor[]) {
    this.#colors = colors ?? [];
    this.#render();
  }

  /**
   * Draws icons by their Lucide names, for each element on lines of its own to start with one: the
   * icon its visualizer names, or one for its kind. The host chooses how icons are drawn and loaded;
   * without it, no element has an icon.
   */
  get icons(): NxViewerIcons | undefined {
    return this.#icons;
  }

  set icons(icons: NxViewerIcons | undefined) {
    this.#icons = typeof icons === "function" ? icons : undefined;
    this.#render();
  }

  /**
   * The keys of what is shown folded: a group as its header alone, a declaration as its first line,
   * and a run of the declarations that support the document (its imports, types, visualizers,
   * components with no body of their own and functions) as one line saying what it holds. Until the
   * host or the reader folds anything, those runs and their declarations are folded and the rest is
   * open, so the document's own content comes first. A host keeps the keys to show a document as the
   * reader left it, and sets `undefined` to go back to the defaults.
   */
  get folded(): readonly string[] {
    return [...this.#folded];
  }

  set folded(keys: readonly string[] | undefined) {
    // A host handing back what the reader just folded changes nothing, and redraws nothing.
    const same =
      keys !== undefined &&
      this.#chosenFolded !== undefined &&
      keys.length === this.#chosenFolded.size &&
      keys.every((key) => this.#chosenFolded!.has(key));
    if (same) {
      return;
    }
    this.#chosenFolded = keys === undefined ? undefined : new Set(keys);
    this.#render();
  }

  /** Folds everything that can fold. */
  collapseAll(): void {
    // What can fold inside a folded unit is not drawn, so it is found with everything unfolded.
    this.#chosenFolded = new Set();
    this.#render();
    this.#setFolded(new Set(this.#foldKeys()));
  }

  /** Unfolds everything. */
  expandAll(): void {
    this.#setFolded(new Set());
  }

  /** Whether the tree is out of date: the reading is muted and badged, and still works. */
  get stale(): boolean {
    return this.hasAttribute("stale");
  }

  set stale(stale: boolean) {
    this.toggleAttribute("stale", stale);
    // Set during an upgrade, the attribute arrives without `attributeChangedCallback`.
    this.#badge.hidden = !stale;
  }

  /**
   * The selected node's key. Setting it selects that node and scrolls it into view, without firing
   * `nx-select`; a key the tree does not hold clears it.
   */
  get selection(): string | undefined {
    return this.#selection;
  }

  set selection(key: string | undefined) {
    this.#select(key, false);
  }

  /**
   * The first node drawn at or below the top of what scrolls the reading, and how far below that
   * top it is, in pixels: what a host keeps in view when it shows the document another way, as in
   * an editor beside the reading. Nothing when no node starts in view.
   */
  keyAtTop(): { key: string; top: number } | undefined {
    const port = this.#scrollPort();
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      if (element.getClientRects().length === 0) {
        continue;
      }
      const top = element.getBoundingClientRect().top - port.top;
      if (top >= -0.5) {
        return top < port.height ? { key: element.dataset["key"]!, top } : undefined;
      }
    }
    return undefined;
  }

  /**
   * How far below the top of what scrolls the reading the node with `key` is drawn, in pixels,
   * negative above it: the node itself, or the nearest node holding it that is drawn, as a folded
   * group. Nothing when neither is drawn.
   */
  topOf(key: string): number | undefined {
    const element = this.#drawnFor(key);
    return element === undefined ? undefined : element.getBoundingClientRect().top - this.#scrollPort().top;
  }

  /**
   * Scrolls the reading so the node with `key`, or the nearest node holding it that is drawn, is
   * `top` pixels below the top of what scrolls it, as far as the reading can scroll.
   */
  scrollToKey(key: string, top = 0): void {
    const now = this.topOf(key);
    if (now !== undefined) {
      this.#scrollPort().element.scrollBy({ top: now - top, behavior: "instant" as ScrollBehavior });
    }
  }

  /** What draws the node with `key`: its own rendering, or the nearest drawn node holding it. */
  #drawnFor(key: string): HTMLElement | undefined {
    const index = this.#index;
    let at = index?.byKey.get(key);
    while (index !== undefined && at !== undefined) {
      const drawn = this.#rendered.get(index.tree.nodes[at]!.key);
      if (drawn !== undefined && drawn.getClientRects().length > 0) {
        return drawn;
      }
      at = index.tree.nodes[at]!.parent;
    }
    return undefined;
  }

  /**
   * What scrolls the reading, whether or not it has anything to scroll now, and the top and height
   * of what it shows: the nearest ancestor that scrolls, or the page.
   */
  #scrollPort(): { element: Element; top: number; height: number } {
    for (let at: Element | null = this.parentElement; at !== null; at = at.parentElement) {
      const overflow = getComputedStyle(at).overflowY;
      if (overflow === "auto" || overflow === "scroll") {
        return { element: at, top: at.getBoundingClientRect().top + at.clientTop, height: at.clientHeight };
      }
    }
    const page = this.ownerDocument.scrollingElement ?? this.ownerDocument.documentElement;
    return { element: page, top: 0, height: this.ownerDocument.defaultView?.innerHeight ?? page.clientHeight };
  }

  attributeChangedCallback(name: string): void {
    if (name === "stale") {
      this.#badge.hidden = !this.stale;
    }
  }

  #render(): void {
    const tree = this.#tree;
    this.#rendered.clear();
    this.#hoveredUnit = undefined;
    this.#body.replaceChildren();
    this.#index = undefined;
    if (tree === undefined) {
      return;
    }
    const index = indexTree(tree, this.#text);
    this.#index = index;
    this.#runs = readingRuns(index);
    this.#folded = this.#chosenFolded ?? new Set(defaultFolded(index));
    const draw = (wrapped?: ReadonlySet<string>): void => {
      this.#body.replaceChildren(
        renderSource(this.ownerDocument, index, this.#colors, { wrapped, icons: this.#icons, folded: this.#folded, turning: this.#turning })
      );
    };
    draw();
    // Each element whose properties do not fit on one line gives each a row, drawn again until all
    // left on lines fit. Hidden or unattached, nothing is measured, and the size the reading takes
    // when shown draws it again.
    this.#fitWidth = this.#body.clientWidth;
    const wrapped = new Set<string>();
    for (let pass = 0; pass < 4 && this.#fitWidth > 0; pass++) {
      const found = this.#wrapped();
      if (found.length === 0) {
        break;
      }
      found.forEach((key) => wrapped.add(key));
      draw(wrapped);
    }
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      this.#rendered.set(element.dataset["key"]!, element);
    }
    // Keep the selection across a new tree when the node is still there. A fold asked for since,
    // by the reader or the host, is kept: when it hides the selection, the selection moves to the
    // folded unit that hides it, as a tree's does, or is cleared when no drawn unit holds it.
    const selection = this.#selection;
    this.#selection = undefined;
    if (selection !== undefined && index.byKey.has(selection)) {
      if (this.#rendered.has(selection)) {
        this.#select(selection, false, false);
      } else {
        const holder = [...this.#folded]
          .filter((folded) => this.#rendered.has(folded) && this.#isInside(selection, folded))
          .sort((a, b) => (this.#isInside(a, b) ? -1 : this.#isInside(b, a) ? 1 : 0))[0];
        if (holder !== undefined) {
          this.#select(holder, true, false);
        }
      }
    }
    this.#body.append(this.#highlights);
    this.#paintHighlights();
    this.#turnChevron();
    // Fonts that arrive later change where the rows fall, and what fits on a line.
    const remeasure = this.ownerDocument.fonts !== undefined && this.ownerDocument.fonts.status !== "loaded";
    void this.ownerDocument.fonts?.ready.then(() => (remeasure ? this.#remeasure() : this.#paintHighlights()));
  }

  /** Draws the reading again, measured anew, keeping the node at the top where it was. */
  #remeasure(): void {
    clearTimeout(this.#remeasureTimer);
    const top = this.keyAtTop();
    this.#render();
    if (top !== undefined) {
      this.scrollToKey(top.key, top.top);
    }
  }

  /**
   * The keys of the elements measured whose line, or whose group's header, takes more than one line,
   * or, for a folded group, whose header runs past its line, since it is cut rather than wrapped.
   */
  #wrapped(): string[] {
    const source = this.#body.querySelector<HTMLElement>(".source");
    if (source === null) {
      return [];
    }
    const lineHeight = parseFloat(getComputedStyle(source).lineHeight) || 18;
    const keys: string[] = [];
    for (const element of this.#body.querySelectorAll<HTMLElement>(".fit-check")) {
      const target = element.classList.contains("sheet") ? element.querySelector<HTMLElement>(":scope > .sheet-head") : element;
      if (target === null) {
        continue;
      }
      if (element.classList.contains("folded")) {
        if (target.scrollWidth > target.clientWidth + 1) {
          keys.push(element.dataset["key"]!);
        }
        continue;
      }
      // An inline line has a box to each line it falls on; a block has one, as tall as its lines.
      const rects = [...target.getClientRects()];
      const lines = rects.length > 1 ? new Set(rects.map((rect) => Math.round(rect.top))).size : (rects[0]?.height ?? 0) / lineHeight;
      if (lines > 1.5) {
        keys.push(element.dataset["key"]!);
      }
    }
    return keys;
  }

  /** The unit `target` is in, or the group it falls in between rows, or nothing. */
  #unitOf(target: Element | null | undefined): HTMLElement | undefined {
    const found = target?.closest?.(UNITS) as HTMLElement | null | undefined;
    if (found !== null && found !== undefined && this.#body.contains(found)) {
      return found.classList.contains("sheet-head") ? (found.parentElement as HTMLElement) : found;
    }
    // Room between a group's rows, or beside them, belongs to the group.
    const group = target?.closest?.(".sheet") as HTMLElement | null | undefined;
    return group !== null && group !== undefined && this.#body.contains(group) ? group : undefined;
  }

  /**
   * Turns the chevron of the fold just toggled, which the reading draws where it was, to where it is,
   * on the next frame.
   */
  #turnChevron(): void {
    this.#turning = undefined;
    const toggles = [...this.#body.querySelectorAll<HTMLElement>(".fold.was-folded, .fold.was-open")];
    if (toggles.length === 0 || typeof requestAnimationFrame === "undefined") {
      toggles.forEach((toggle) => toggle.classList.remove("was-folded", "was-open"));
      return;
    }
    requestAnimationFrame(() => requestAnimationFrame(() => toggles.forEach((toggle) => toggle.classList.remove("was-folded", "was-open"))));
  }

  /** The keys of every fold control drawn, in reading order. */
  #foldKeys(within: Element = this.#body): string[] {
    return [...within.querySelectorAll<HTMLElement>(".fold")].map((toggle) => toggle.dataset["foldKey"]!);
  }

  /** Folds what `folded` names and nothing else, redraws, and tells the host. */
  #setFolded(folded: Set<string>): void {
    this.#chosenFolded = folded;
    this.#render();
    const detail: NxFoldDetail = { folded: [...folded], selection: this.#selection };
    this.dispatchEvent(new CustomEvent(NX_FOLD_EVENT, { detail, bubbles: true, composed: true }));
  }

  /**
   * Folds or unfolds the unit with `key`. With `deep`, everything inside it goes the same way, as an
   * Alt+click does in a design tool's layers.
   */
  #toggleFold(key: string, deep: boolean): void {
    const folded = new Set(this.#folded);
    const folding = !folded.has(key);
    this.#turning = { key, wasFolded: !folding };
    // The unit stays where it is on the screen: what folds or opens is below it, and the reading
    // is scrolled back by however far its line moved.
    const toggleOf = (): HTMLElement | undefined =>
      [...this.#body.querySelectorAll<HTMLElement>(".fold")].find((toggle) => toggle.dataset["foldKey"] === key);
    const before = toggleOf()?.getBoundingClientRect().top;
    if (folding && deep) {
      // What can fold inside a folded unit is not drawn, so it is found with the unit opened all
      // the way down, as `collapseAll` finds everything.
      const open = new Set([...folded].filter((other) => other !== key && !this.#isInside(other, key)));
      this.#chosenFolded = open;
      this.#render();
      const unit = toggleOf()?.closest<HTMLElement>(".sheet, .fold-declaration, .fold-run");
      for (const inner of unit !== null && unit !== undefined ? this.#foldKeys(unit) : [key]) {
        folded.add(inner);
      }
    } else if (folding) {
      folded.add(key);
    } else {
      folded.delete(key);
      if (deep) {
        for (const other of [...folded]) {
          if (this.#isInside(other, key)) {
            folded.delete(other);
          }
        }
      }
    }
    this.#setFolded(folded);
    const after = toggleOf()?.getBoundingClientRect().top;
    if (before !== undefined && after !== undefined && after !== before) {
      this.#scroller()?.scrollBy({ top: after - before, behavior: "instant" as ScrollBehavior });
    }
  }

  /** What scrolls the reading: the nearest ancestor of the element that scrolls, or the page. */
  #scroller(): Element | undefined {
    for (let at: Element | null = this.parentElement; at !== null; at = at.parentElement) {
      const overflow = getComputedStyle(at).overflowY;
      if ((overflow === "auto" || overflow === "scroll") && at.scrollHeight > at.clientHeight) {
        return at;
      }
    }
    return this.ownerDocument.scrollingElement ?? undefined;
  }

  /** Whether the node with `key` lies inside the node with `outer`. */
  #isInside(key: string, outer: string): boolean {
    const index = this.#index;
    const run = this.#runs.find((candidate) => candidate.key === outer);
    if (run !== undefined) {
      return run.nodes.some((top) => top === index?.byKey.get(key) || this.#isInside(key, index!.tree.nodes[top]!.key));
    }
    const target = index?.byKey.get(outer);
    let at = index?.byKey.get(key);
    while (index !== undefined && at !== undefined) {
      at = index.tree.nodes[at]?.parent;
      if (at === target) {
        return true;
      }
    }
    return false;
  }

  #setHoveredUnit(unit: HTMLElement | undefined): void {
    if (unit !== this.#hoveredUnit) {
      this.#hoveredUnit = unit;
      this.#paintHighlights();
    }
  }

  /**
   * Draws the selected unit and the one under the pointer as rows behind the text, from just before
   * the unit's start to near the reading's right edge, as a list's rows are, or, for an element
   * inside a line, to just past its end. A selected group's first line is drawn stronger than the
   * rest of it, and the row under the pointer, lighter over a selection.
   */
  #paintHighlights(): void {
    const layer = this.#highlights;
    layer.replaceChildren();
    if (!this.#body.contains(layer)) {
      return;
    }
    const frame = this.#body.getBoundingClientRect();
    if (frame.width === 0) {
      return;
    }
    const draw = (from: HTMLElement, rows: DOMRect, className: string): void => {
      const start = (from.matches(".sheet-row") ? from.querySelector<HTMLElement>(":scope > .sheet-name") : from) ?? from;
      // A block's later lines start further left, at its source indentation; it starts where its
      // first line does.
      const first = from.matches(".block-line") ? from.getClientRects()[0] : undefined;
      // An element inside a line, as a property's value, ends where it does, unless it wraps, and is
      // drawn tighter, so it stays clear of the text beside it.
      const inline = from.matches(".line-element:not(.alone)") ? from.getClientRects() : undefined;
      const tight = inline?.length === 1 ? inline[0] : undefined;
      const pad = tight === undefined ? 4 : 1;
      const left = (first ?? start.getBoundingClientRect()).left - frame.left - pad + this.#body.scrollLeft;
      const box = this.ownerDocument.createElement("div");
      box.className = `highlight ${className}`;
      box.style.left = `${left}px`;
      box.style.top = `${rows.top - frame.top + this.#body.scrollTop}px`;
      const right = tight === undefined ? frame.width - 8 : tight.right - frame.left + pad + this.#body.scrollLeft;
      box.style.width = `${Math.max(0, right - left)}px`;
      box.style.height = `${rows.height}px`;
      layer.append(box);
    };
    // A group's first line: a sheet's header, or a block's first line.
    const firstLine = (group: HTMLElement): DOMRect | undefined =>
      group.matches(".sheet")
        ? group.querySelector<HTMLElement>(":scope > .sheet-head")?.getBoundingClientRect()
        : group.matches(".block-line")
          ? group.getClientRects()[0]
          : undefined;
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const unit = selected?.matches(DRAWN_UNITS) === true ? selected : undefined;
    if (unit !== undefined) {
      const head = firstLine(unit);
      if (head !== undefined) {
        draw(unit, unit.getBoundingClientRect(), "selected-body");
        draw(unit, head, "selected");
      } else {
        draw(unit, unit.getBoundingClientRect(), "selected");
      }
    }
    const hovered = this.#hoveredUnit;
    if (hovered !== undefined && this.#body.contains(hovered)) {
      const within = unit !== undefined && unit.contains(hovered);
      draw(hovered, firstLine(hovered) ?? hovered.getBoundingClientRect(), within ? "hovered in-selection" : "hovered");
    }
  }

  /** Every unit, in reading order. */
  #units(): HTMLElement[] {
    const units: HTMLElement[] = [];
    for (const found of this.#body.querySelectorAll<HTMLElement>(UNITS)) {
      const unit = this.#unitOf(found);
      if (unit !== undefined && !units.includes(unit)) {
        units.push(unit);
      }
    }
    return units;
  }

  /**
   * Selects the next or previous unit, or the one that holds the selected one, or the first it holds.
   * A selection that is not a unit, as a node the host selects inside a row, moves from the unit that
   * holds it, or from where it stands when no unit holds it.
   */
  #stepUnit(move: "next" | "previous" | "out" | "in"): void {
    // A group folds and unfolds before the arrows move past it, as a tree's nodes do.
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const toggle = selected?.matches(".sheet") === true ? selected.querySelector<HTMLElement>(":scope > .sheet-head > .fold") : null;
    if (toggle !== null && toggle !== undefined) {
      const key = toggle.dataset["foldKey"]!;
      if ((move === "in" && this.#folded.has(key)) || (move === "out" && !this.#folded.has(key))) {
        this.#toggleFold(key, false);
        return;
      }
    }
    const units = this.#units();
    if (units.length === 0) {
      return;
    }
    const current = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const holder = current === undefined || units.includes(current) ? current : this.#unitOf(current);
    const at = holder === undefined ? -1 : units.indexOf(holder);
    let next: HTMLElement | undefined;
    if (current === undefined) {
      next = move === "previous" ? units[units.length - 1] : units[0];
    } else if (move === "in") {
      next = units.find((unit) => unit !== current && current.contains(unit));
    } else if (move === "out") {
      next = holder !== current ? holder : units.slice(0, at).reverse().find((unit) => unit.contains(current));
    } else if (at >= 0) {
      next = units[Math.max(0, Math.min(units.length - 1, at + (move === "next" ? 1 : -1)))];
    } else if (move === "next") {
      // Outside every unit, as a type's field: the first unit after it.
      next = units.find((unit) => (current.compareDocumentPosition(unit) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0);
    } else {
      next = [...units].reverse().find((unit) => (current.compareDocumentPosition(unit) & Node.DOCUMENT_POSITION_PRECEDING) !== 0);
    }
    if (next !== undefined) {
      this.#select(keyOf(next), true);
      next.scrollIntoView?.({ block: "nearest" });
    }
  }

  /**
   * Selects the node with `key`. With `reveal`, as when the reader or the host chooses it, what
   * folds it away is unfolded and it is scrolled into view; without, as when the reading is drawn
   * again, the folds stay as they are.
   */
  #select(key: string | undefined, fire: boolean, reveal = true): void {
    for (const element of this.#body.querySelectorAll(".selected")) {
      element.classList.remove("selected");
      element.removeAttribute("aria-current");
    }
    queueMicrotask(() => this.#paintHighlights());
    const index = this.#index;
    const at = key === undefined ? undefined : index?.byKey.get(key);
    if (index === undefined || key === undefined || at === undefined) {
      this.#selection = undefined;
      return;
    }
    // A node folded away is unfolded to, so what is selected can be seen.
    const hiding = reveal ? [...this.#folded].filter((folded) => folded !== key && this.#isInside(key, folded)) : [];
    if (hiding.length > 0) {
      this.#selection = key;
      const folded = new Set(this.#folded);
      hiding.forEach((folded_) => folded.delete(folded_));
      this.#setFolded(folded);
      return;
    }
    this.#selection = key;
    const original = this.#rendered.get(key);
    original?.classList.add("selected");
    original?.setAttribute("aria-current", "true");
    if (fire) {
      if (original !== undefined) {
        const words = spoken(original);
        this.#announcer.textContent = `Selected ${words.length > 140 ? `${words.slice(0, 139)}…` : words}`;
      }
      const node = nodeOf(index, at);
      const detail: NxSelectDetail = { key, role: node.role, range: node.range };
      this.dispatchEvent(new CustomEvent(NX_SELECT_EVENT, { detail, bubbles: true, composed: true }));
    } else if (reveal) {
      original?.scrollIntoView?.({ block: "nearest" });
    }
  }

  #onClick(event: MouseEvent): void {
    const target = event.target as Element;
    const toggle = (target.closest?.(".fold-run-head")?.querySelector(".fold") ?? target.closest?.(".fold")) as
      | HTMLElement
      | null
      | undefined;
    if (toggle !== null && toggle !== undefined && this.#body.contains(toggle)) {
      this.#toggleFold(toggle.dataset["foldKey"]!, event.altKey);
      return;
    }
    // A drag that selected text is a text selection, not a choice of node.
    // Chromium reports a selection inside a shadow root only through the root itself.
    const root = this.shadowRoot as (ShadowRoot & { getSelection?(): Selection | null }) | null;
    const textSelection = root?.getSelection?.() ?? this.ownerDocument.getSelection?.();
    if (textSelection !== null && textSelection !== undefined && !textSelection.isCollapsed) {
      return;
    }
    // A click chooses the unit it falls in: the element, row or line, not a word in it. Outside
    // every unit, as in a function's body, it chooses the smallest node there.
    const chosen = this.#unitOf(target) ?? (target.closest?.("[data-key]") as HTMLElement | null | undefined);
    if (chosen !== null && chosen !== undefined && this.#body.contains(chosen)) {
      this.#select(keyOf(chosen), true);
    }
  }

  #onKeyDown(event: KeyboardEvent): void {
    const target = event.target as Element;
    if (!this.#body.contains(target)) {
      return;
    }
    // The units read as a tree: up and down walk its rows, left goes to the unit that holds this one,
    // and right into the first one it holds.
    const moves = { ArrowDown: "next", ArrowUp: "previous", ArrowLeft: "out", ArrowRight: "in" } as const;
    const move = moves[event.key as keyof typeof moves];
    if (move !== undefined) {
      event.preventDefault();
      this.#stepUnit(move);
    }
  }
}

/**
 * Defines `<nx-viewer>` in `registry`, the page's by default. Importing this module already does
 * it where there is a DOM; calling it again is harmless.
 */
export function defineNxViewerElement(
  registry: CustomElementRegistry | undefined = globalThis.customElements
): void {
  if (registry !== undefined && registry.get(NX_VIEWER_TAG) === undefined) {
    registry.define(NX_VIEWER_TAG, NxViewerElement);
  }
}

defineNxViewerElement();

declare global {
  interface HTMLElementTagNameMap {
    "nx-viewer": NxViewerElement;
  }
}
