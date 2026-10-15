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

import { renderSource, type NxViewerColor, type NxViewerIcons } from "./source.js";
import { styles } from "./styles.js";
import { indexTree, nodeOf, type TreeIndex } from "./tree.js";

export { nodeAtSpan } from "./tree.js";
export type { NxViewerColor, NxViewerIcons } from "./source.js";

/** The tag the element is defined under. */
export const NX_VIEWER_TAG = "nx-viewer";

/** The event fired when a reader selects a node. */
export const NX_SELECT_EVENT = "nx-select";

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
const PROPERTIES = ["tree", "text", "colors", "icons", "selection", "stale"] as const;

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
 * `tree`, `text`, `colors`, `icons` and `selection`.</para>
 */
export class NxViewerElement extends ElementBase {
  static observedAttributes = ["stale"];

  #tree: SourceTree | undefined;
  #text = "";
  #colors: readonly NxViewerColor[] = [];
  #icons: NxViewerIcons | undefined;
  #index: TreeIndex | undefined;
  #selection: string | undefined;
  /** Each key's rendering. */
  #rendered = new Map<string, HTMLElement>();
  /** The unit under the pointer, highlighted as a row. */
  #hoveredUnit: HTMLElement | undefined;

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
      new ResizeObserver(() => this.#paintHighlights()).observe(this.#body);
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
   * icon its viewer names, or one for its kind. The host chooses how icons are drawn and loaded;
   * without it, no element has an icon.
   */
  get icons(): NxViewerIcons | undefined {
    return this.#icons;
  }

  set icons(icons: NxViewerIcons | undefined) {
    this.#icons = typeof icons === "function" ? icons : undefined;
    this.#render();
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
    this.#body.append(renderSource(this.ownerDocument, index, this.#colors, { icons: this.#icons }));
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      this.#rendered.set(element.dataset["key"]!, element);
    }
    // Keep the selection across a new tree when the node is still there.
    const selection = this.#selection;
    this.#selection = undefined;
    this.#select(selection !== undefined && index.byKey.has(selection) ? selection : undefined, false);
    this.#body.append(this.#highlights);
    this.#paintHighlights();
    // Fonts that arrive later change where the rows fall.
    void this.ownerDocument.fonts?.ready.then(() => this.#paintHighlights());
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

  #setHoveredUnit(unit: HTMLElement | undefined): void {
    if (unit !== this.#hoveredUnit) {
      this.#hoveredUnit = unit;
      this.#paintHighlights();
    }
  }

  /**
   * Draws the selected unit and the one under the pointer as rows behind the text, from just before
   * the unit's start to near the reading's right edge, as a list's rows are. A selected group's
   * first line is drawn stronger than the rest of it, and the row under the pointer, lighter over a
   * selection.
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
      const left = (first ?? start.getBoundingClientRect()).left - frame.left - 4 + this.#body.scrollLeft;
      const box = this.ownerDocument.createElement("div");
      box.className = `highlight ${className}`;
      box.style.left = `${left}px`;
      box.style.top = `${rows.top - frame.top + this.#body.scrollTop}px`;
      box.style.width = `${Math.max(0, frame.width - 8 - left)}px`;
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

  #select(key: string | undefined, fire: boolean): void {
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
    } else {
      original?.scrollIntoView?.({ block: "nearest" });
    }
  }

  #onClick(event: MouseEvent): void {
    const target = event.target as Element;
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
