/**
 * `<nx-viewer>`: an NX document's source tree as a reading view.
 *
 * <para>The element takes a `SourceTree`, as the language service's `sourceTree` query answers it,
 * and the text of the document it describes. Syntax becomes style and meaning stays visible: each
 * element reads as a card, each condition as a sentence, and every rendered piece carries its
 * node's key and opens to the exact text behind it. It has no framework dependency and keeps its
 * markup and styles in a shadow root.</para>
 */
import type { EditorRange, SourceRole, SourceTree } from "@nx-lang/language-protocol";

import { declarationHover, propertyHover } from "./hover.js";
import { renderDocument, renderNode, valueNodeOf, type RenderContext } from "./render.js";
import { styles } from "./styles.js";
import { indexTree, nodeOf, sourceTextOf, type TreeIndex } from "./tree.js";

export { nodeAtSpan } from "./tree.js";
export { inWords, eventWords } from "./words.js";

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
const PROPERTIES = ["tree", "text", "ghosts", "selection", "stale"] as const;

/** How long the pointer rests on a name before its hover shows, in milliseconds. */
const hoverDelay = 250;

// A module imported where there is no DOM still loads; the element is only defined where
// `HTMLElement` exists.
const ElementBase = (typeof HTMLElement === "undefined" ? class {} : HTMLElement) as typeof HTMLElement;

/** The attributes that carry a rendered node's key: an original, a copy, a later part of a run. */
const KEYED = "[data-key], [data-ref-key], [data-part-of]";

function keyOf(element: Element): string | undefined {
  const data = (element as HTMLElement).dataset;
  return data["key"] ?? data["refKey"] ?? data["partOf"];
}

/**
 * The `<nx-viewer>` element.
 *
 * <para>The attribute `stale` mirrors the property of the same name. Everything else is a property:
 * `tree`, `text`, `ghosts` and `selection`.</para>
 */
export class NxViewerElement extends ElementBase {
  static observedAttributes = ["stale"];

  #tree: SourceTree | undefined;
  #text = "";
  #ghosts = true;
  #index: TreeIndex | undefined;
  #selection: string | undefined;
  /** Each key's original rendering. */
  #rendered = new Map<string, HTMLElement>();
  #hovered: HTMLElement | undefined;
  #hoverTimer: ReturnType<typeof setTimeout> | undefined;
  #shownSource: string | undefined;

  readonly #frame: HTMLDivElement;
  readonly #toolbar: HTMLDivElement;
  readonly #badge: HTMLSpanElement;
  readonly #selectionSource: HTMLButtonElement;
  readonly #body: HTMLDivElement;
  readonly #hover: HTMLDivElement;
  readonly #sourcePanel: HTMLDivElement;
  readonly #sourceTitle: HTMLSpanElement;
  readonly #sourceText: HTMLPreElement;

  constructor() {
    super();
    const shadow = this.attachShadow({ mode: "open" });
    const document = this.ownerDocument;

    const style = document.createElement("style");
    style.textContent = styles;

    this.#frame = document.createElement("div");
    this.#frame.className = "frame";

    this.#toolbar = document.createElement("div");
    this.#toolbar.className = "toolbar";
    this.#badge = document.createElement("span");
    this.#badge.className = "badge";
    this.#badge.textContent = "Out of date";
    this.#badge.hidden = true;
    this.#selectionSource = document.createElement("button");
    this.#selectionSource.type = "button";
    this.#selectionSource.className = "selection-source";
    this.#selectionSource.textContent = "Source of selection";
    this.#selectionSource.hidden = true;
    this.#toolbar.append(this.#badge, this.#selectionSource);

    this.#body = document.createElement("div");
    this.#body.className = "body";

    this.#hover = document.createElement("div");
    this.#hover.className = "hover";
    this.#hover.setAttribute("role", "tooltip");
    this.#hover.hidden = true;

    this.#sourcePanel = document.createElement("div");
    this.#sourcePanel.className = "source-panel";
    this.#sourcePanel.setAttribute("role", "dialog");
    this.#sourcePanel.setAttribute("aria-label", "Source");
    this.#sourcePanel.hidden = true;
    const bar = document.createElement("div");
    bar.className = "source-bar";
    this.#sourceTitle = document.createElement("span");
    const close = document.createElement("button");
    close.type = "button";
    close.className = "source-close";
    close.textContent = "Close";
    bar.append(this.#sourceTitle, close);
    this.#sourceText = document.createElement("pre");
    this.#sourcePanel.append(bar, this.#sourceText);

    this.#frame.append(this.#toolbar, this.#body, this.#hover, this.#sourcePanel);
    shadow.append(style, this.#frame);

    close.addEventListener("click", () => this.hideSource());
    this.#selectionSource.addEventListener("click", () => {
      if (this.#selection !== undefined) {
        this.showSource(this.#selection);
      }
    });
    this.#body.addEventListener("click", (event) => this.#onClick(event));
    this.#body.addEventListener("keydown", (event) => this.#onKeyDown(event));
    this.#body.addEventListener("pointerover", (event) => this.#onPointerOver(event));
    this.#body.addEventListener("pointerleave", () => this.#setHovered(undefined));
    this.#body.addEventListener("focusin", (event) => this.#onFocusIn(event));

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

  /** The text of the document the tree describes, for showing a node's source. */
  get text(): string {
    return this.#text;
  }

  set text(text: string) {
    this.#text = text ?? "";
    this.#render();
  }

  /** Whether omitted defaults show as ghost rows; true unless set false. */
  get ghosts(): boolean {
    return this.#ghosts;
  }

  set ghosts(ghosts: boolean) {
    this.#ghosts = ghosts !== false;
    this.#render();
  }

  /** Whether the tree is out of date: the reading is muted and badged, and still works. */
  get stale(): boolean {
    return this.hasAttribute("stale");
  }

  set stale(stale: boolean) {
    this.toggleAttribute("stale", stale);
  }

  /**
   * The selected node's key. Setting it selects that node, opening anything folded around it and
   * scrolling it into view, without firing `nx-select`; a key the tree does not hold clears it.
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

  /** The key of the node whose source is shown, if any. */
  get shownSource(): string | undefined {
    return this.#shownSource;
  }

  /** Shows the exact source text of the node with `key`, beside its rendering. */
  showSource(key: string): void {
    const index = this.#index;
    const at = index?.byKey.get(key);
    if (index === undefined || at === undefined) {
      return;
    }
    this.#setHovered(undefined);
    this.#shownSource = key;
    this.#sourceTitle.textContent = `Source · ${nodeOf(index, at).role}`;
    this.#sourceText.textContent = sourceTextOf(index, at);
    this.#sourcePanel.hidden = false;
    this.#position(this.#sourcePanel, this.#rendered.get(key));
  }

  /** Hides the source shown by `showSource`. */
  hideSource(): void {
    this.#shownSource = undefined;
    this.#sourcePanel.hidden = true;
  }

  #render(): void {
    const tree = this.#tree;
    this.#setHovered(undefined);
    this.hideSource();
    this.#rendered.clear();
    this.#body.replaceChildren();
    this.#index = undefined;
    if (tree === undefined) {
      this.#selectionSource.hidden = true;
      return;
    }
    const index = indexTree(tree, this.#text);
    this.#index = index;
    const context: RenderContext = { document: this.ownerDocument, index, ghosts: this.#ghosts, copy: false };
    this.#body.append(renderDocument(context));
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      this.#rendered.set(element.dataset["key"]!, element);
    }
    // Keep the selection across a new tree when the node is still there.
    const selection = this.#selection;
    this.#selection = undefined;
    this.#select(selection !== undefined && index.byKey.has(selection) ? selection : undefined, false);
  }

  #select(key: string | undefined, fire: boolean): void {
    for (const element of this.#body.querySelectorAll(".selected")) {
      element.classList.remove("selected");
    }
    const index = this.#index;
    const at = key === undefined ? undefined : index?.byKey.get(key);
    if (index === undefined || key === undefined || at === undefined) {
      this.#selection = undefined;
      this.#selectionSource.hidden = true;
      return;
    }
    this.#selection = key;
    this.#selectionSource.hidden = false;
    const original = this.#rendered.get(key);
    original?.classList.add("selected");
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-ref-key], [data-part-of]")) {
      if (keyOf(element) === key) {
        element.classList.add("selected");
      }
    }
    if (fire) {
      const node = nodeOf(index, at);
      const detail: NxSelectDetail = { key, role: node.role, range: node.range };
      this.dispatchEvent(new CustomEvent(NX_SELECT_EVENT, { detail, bubbles: true, composed: true }));
    } else if (original !== undefined) {
      for (let parent = original.parentElement; parent !== null; parent = parent.parentElement) {
        if (parent.tagName === "DETAILS") {
          (parent as HTMLDetailsElement).open = true;
        }
      }
      original.scrollIntoView?.({ block: "nearest" });
    }
  }

  #onClick(event: MouseEvent): void {
    const target = event.target as Element;
    const button = target.closest?.("button");
    if (button !== null && button !== undefined) {
      if (button.classList.contains("source-toggle")) {
        const key = button.dataset["sourceFor"]!;
        if (this.#shownSource === key) {
          this.hideSource();
        } else {
          this.showSource(key);
        }
      } else if (button.classList.contains("expand")) {
        this.#toggleExpansion(button);
      }
      return;
    }
    // A drag that selected text is a text selection, not a choice of node.
    const textSelection = this.ownerDocument.getSelection?.();
    if (textSelection !== null && textSelection !== undefined && !textSelection.isCollapsed) {
      return;
    }
    const keyed = target.closest?.(KEYED);
    if (keyed !== null && keyed !== undefined && this.#body.contains(keyed)) {
      this.#select(keyOf(keyed), true);
    }
  }

  #onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Enter" && !(event.target as Element).closest?.("button")) {
      const keyed = (event.target as Element).closest?.(KEYED);
      if (keyed !== null && keyed !== undefined) {
        event.preventDefault();
        this.#select(keyOf(keyed), true);
      }
    } else if (event.key === "Escape") {
      this.#setHovered(undefined);
      this.hideSource();
    }
  }

  /** Shows or hides a reference's target in place, rendered as a copy of its original. */
  #toggleExpansion(button: HTMLButtonElement): void {
    const wrapper = button.parentElement;
    const index = this.#index;
    if (wrapper === null || index === undefined) {
      return;
    }
    const existing = wrapper.querySelector<HTMLElement>(":scope > .expansion");
    if (existing !== null) {
      existing.hidden = !existing.hidden;
      button.setAttribute("aria-expanded", String(!existing.hidden));
      button.textContent = existing.hidden ? "▸" : "▾";
      return;
    }
    const key = keyOf(wrapper);
    const at = key === undefined ? undefined : index.byKey.get(key);
    const entry = at === undefined ? undefined : index.tree.declarations[nodeOf(index, at).declaration ?? -1];
    if (entry?.valueRange === undefined) {
      return;
    }
    const context: RenderContext = { document: this.ownerDocument, index, ghosts: this.#ghosts, copy: true };
    const target = valueNodeOf(context, entry.valueRange);
    const rendered = target === undefined ? undefined : renderNode(context, target);
    if (rendered === undefined) {
      return;
    }
    const expansion = this.ownerDocument.createElement("div");
    expansion.className = "expansion";
    expansion.append(rendered);
    wrapper.append(expansion);
    button.setAttribute("aria-expanded", "true");
    button.textContent = "▾";
  }

  #hoverTarget(target: Element | null): HTMLElement | undefined {
    const found = target?.closest?.(".label, .ref, .kind");
    return found === null || found === undefined || !this.#body.contains(found) ? undefined : (found as HTMLElement);
  }

  #onPointerOver(event: PointerEvent): void {
    this.#setHovered(this.#hoverTarget(event.target as Element));
  }

  #onFocusIn(event: FocusEvent): void {
    const target = this.#hoverTarget(event.target as Element);
    if (target !== undefined) {
      this.#setHovered(target, true);
    }
  }

  #setHovered(target: HTMLElement | undefined, immediately = false): void {
    if (target === this.#hovered) {
      return;
    }
    this.#hovered?.classList.remove("hovered");
    this.#hovered = target;
    clearTimeout(this.#hoverTimer);
    this.#hover.hidden = true;
    if (target === undefined) {
      return;
    }
    target.classList.add("hovered");
    if (immediately) {
      this.#showHover(target);
    } else {
      this.#hoverTimer = setTimeout(() => this.#showHover(target), hoverDelay);
    }
  }

  /** What a hover over `target` says, or nothing. */
  hoverFor(target: HTMLElement): HTMLElement | undefined {
    const index = this.#index;
    if (index === undefined) {
      return undefined;
    }
    const document = this.ownerDocument;
    const owner = target.closest(KEYED);
    const ownerKey = owner === null ? undefined : keyOf(owner);
    const ownerAt = ownerKey === undefined ? undefined : index.byKey.get(ownerKey);
    if (target.classList.contains("label")) {
      const card = target.closest(".card");
      const cardKey = card === null ? undefined : keyOf(card);
      const element = cardKey === undefined ? undefined : index.byKey.get(cardKey);
      if (element === undefined) {
        return undefined;
      }
      // The node the attribute sets the property to, when the row is an attribute's.
      const attribute = ownerAt !== undefined && nodeOf(index, ownerAt).role === "attribute" ? ownerAt : undefined;
      const valueAt = attribute === undefined ? undefined : (index.children[attribute] ?? []).find((child) => nodeOf(index, child).role !== "comment");
      return propertyHover(
        document,
        index,
        element,
        target.dataset["property"] ?? target.textContent ?? "",
        valueAt === undefined ? undefined : nodeOf(index, valueAt)
      );
    }
    if (ownerAt === undefined) {
      return undefined;
    }
    return declarationHover(document, index, nodeOf(index, ownerAt));
  }

  #showHover(target: HTMLElement): void {
    if (this.#hovered !== target) {
      return;
    }
    const content = this.hoverFor(target);
    if (content === undefined) {
      return;
    }
    this.#hover.replaceChildren(content);
    this.#hover.hidden = false;
    this.#position(this.#hover, target);
  }

  #position(panel: HTMLElement, anchor: HTMLElement | undefined): void {
    if (anchor === undefined) {
      panel.style.top = "0px";
      panel.style.left = "0px";
      return;
    }
    const frame = this.#frame.getBoundingClientRect();
    const box = anchor.getClientRects()[0] ?? anchor.getBoundingClientRect();
    panel.style.top = `${box.bottom - frame.top + 4}px`;
    panel.style.left = `${Math.max(0, Math.min(box.left - frame.left, frame.width - panel.offsetWidth))}px`;
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
