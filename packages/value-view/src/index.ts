/**
 * `<nx-value>`: an NX value shown as NX text.
 *
 * <para>The element takes the annotated text `evaluateNx()` returns, highlights it with the grammar
 * the editors use, folds what spans several lines, and explains each part on hover. It has no
 * framework dependency and keeps its markup and styles in a shadow root, so a page only imports
 * this module, creates the element and sets `value`.</para>
 */
import { defaultDescription } from "./describe.js";
import {
  colorize,
  DEFAULT_THEMES,
  loadHighlighter,
  loadedHighlighter,
  type ColoredRange,
  type NxValueHighlighter,
  type NxValueThemes
} from "./highlight.js";
import { renderMarkdown } from "./markdown.js";
import { renderValue, type RenderedValue } from "./render.js";
import { styles } from "./styles.js";
import type { NxValueNode, NxValueSpan, NxValueText } from "./types.js";

export type { NxValueNode, NxValueRole, NxValueSpan, NxValueText } from "./types.js";
export {
  DEFAULT_THEMES,
  NX_LANGUAGE_ID,
  nxShikiLanguage,
  type NxValueHighlighter,
  type NxValueThemes
} from "./highlight.js";
export { defaultDescription } from "./describe.js";

/** The tag the element is defined under. */
export const NX_VALUE_TAG = "nx-value";

/** The event fired when a node with a declaration is chosen, by click or with Enter. */
export const NX_VALUE_NAVIGATE_EVENT = "nx-value-navigate";

/** What the `nx-value-navigate` event carries. */
export interface NxValueNavigateDetail {
  /** The node chosen. */
  readonly node: NxValueNode;
  /** Its index in the value's `nodes`. */
  readonly index: number;
  /** Where the program declares it. */
  readonly declaration: NxValueSpan;
}

/**
 * The host's hover for a node: markdown, or nothing for the element's default. It may answer
 * asynchronously; an answer that arrives after the pointer has moved on is dropped.
 */
export type NxValueDescribe = (
  node: NxValueNode,
  index: number,
  value: NxValueText
) => string | undefined | Promise<string | undefined>;

/** The element's properties, which a host may set before the element is defined. */
const PROPERTIES = ["value", "describe", "highlighter", "themes", "truncated", "stale"] as const;

/**
 * The longest text the element colors. Coloring runs Shiki over the whole text on the main thread,
 * which for the 100,000 characters the playground shows at most took about a third of a second on
 * every change; a value longer than this is shown uncolored, and still folds and hovers.
 */
export const MAX_COLORED_CHARACTERS = 20_000;

/** How long the pointer rests on a node before its hover shows, in milliseconds. */
const hoverDelay = 250;

// A module imported where there is no DOM, a server render or a test that has not set one up,
// still loads. The element is only usable, and only defined, where `HTMLElement` exists.
const ElementBase = (
  typeof HTMLElement === "undefined" ? class {} : HTMLElement
) as typeof HTMLElement;

/**
 * The `<nx-value>` element.
 *
 * <para>Attributes `truncated` and `stale` mirror the properties of the same names. Everything else
 * is a property: `value`, `describe`, `highlighter` and `themes`.</para>
 */
export class NxValueElement extends ElementBase {
  static observedAttributes = ["truncated", "stale"];

  #value: NxValueText | undefined;
  #describe: NxValueDescribe | undefined;
  #highlighter: NxValueHighlighter | undefined;
  #loadedHighlighter: NxValueHighlighter | undefined;
  #themes: NxValueThemes = DEFAULT_THEMES;
  #rendered: RenderedValue | undefined;
  #hovered: number | undefined;
  #hoverTimer: ReturnType<typeof setTimeout> | undefined;
  #hoverRequest = 0;
  #copyTimer: ReturnType<typeof setTimeout> | undefined;

  readonly #pre: HTMLPreElement;
  readonly #frame: HTMLDivElement;
  readonly #notice: HTMLParagraphElement;
  readonly #badge: HTMLSpanElement;
  readonly #copy: HTMLButtonElement;
  readonly #hover: HTMLDivElement;

  constructor() {
    super();
    const shadow = this.attachShadow({ mode: "open" });
    const document = this.ownerDocument;

    const style = document.createElement("style");
    style.textContent = styles;

    this.#frame = document.createElement("div");
    this.#frame.className = "frame";

    const toolbar = document.createElement("div");
    toolbar.className = "toolbar";
    this.#badge = document.createElement("span");
    this.#badge.className = "badge";
    this.#badge.textContent = "Out of date";
    this.#badge.hidden = true;
    this.#copy = document.createElement("button");
    this.#copy.className = "copy";
    this.#copy.type = "button";
    this.#copy.textContent = "Copy";
    this.#copy.setAttribute("aria-label", "Copy the value as NX");
    this.#copy.hidden = true;
    toolbar.append(this.#badge, this.#copy);

    this.#pre = document.createElement("pre");
    this.#pre.tabIndex = 0;
    this.#pre.setAttribute("aria-label", "NX value");

    this.#notice = document.createElement("p");
    this.#notice.className = "notice";
    this.#notice.textContent = "The rest of the value was cut.";
    this.#notice.hidden = true;

    this.#hover = document.createElement("div");
    this.#hover.className = "hover";
    this.#hover.setAttribute("role", "tooltip");
    this.#hover.hidden = true;

    this.#frame.append(toolbar, this.#pre, this.#notice, this.#hover);
    shadow.append(style, this.#frame);

    this.#copy.addEventListener("click", () => void this.#copyText());
    this.#pre.addEventListener("click", (event) => this.#onClick(event));
    this.#pre.addEventListener("pointermove", (event) => this.#onPointerMove(event));
    this.#pre.addEventListener("pointerleave", () => this.#setHovered(undefined));
    this.#pre.addEventListener("keydown", (event) => this.#onKeyDown(event));
    this.#pre.addEventListener("focusin", (event) => this.#onFocusIn(event));
    this.#pre.addEventListener("focusout", (event) => {
      if (!this.#pre.contains(event.relatedTarget as Node | null)) {
        this.#setHovered(undefined);
      }
    });

    // A property set before the element was defined, on an `<nx-value>` in the page's HTML or
    // one a framework rendered before this module loaded, is an own property that hides the
    // accessor. Taking it off and setting it again passes it through the accessor.
    for (const name of PROPERTIES) {
      if (Object.prototype.hasOwnProperty.call(this, name)) {
        const value = (this as Record<string, unknown>)[name];
        delete (this as Record<string, unknown>)[name];
        (this as Record<string, unknown>)[name] = value;
      }
    }
  }

  /** The value to show: `{ text, nodes }`, as `evaluateNx()` returns it. */
  get value(): NxValueText | undefined {
    return this.#value;
  }

  set value(value: NxValueText | undefined) {
    this.#value = value;
    this.#render();
  }

  /** Whether the text was cut short; a notice follows it when set. */
  get truncated(): boolean {
    return this.hasAttribute("truncated");
  }

  set truncated(truncated: boolean) {
    this.toggleAttribute("truncated", truncated);
  }

  /** Whether the value is out of date: it is muted and badged, and still hovers and folds. */
  get stale(): boolean {
    return this.hasAttribute("stale");
  }

  set stale(stale: boolean) {
    this.toggleAttribute("stale", stale);
  }

  /** The host's hover for a node, or nothing for the element's default. */
  get describe(): NxValueDescribe | undefined {
    return this.#describe;
  }

  set describe(describe: NxValueDescribe | undefined) {
    this.#describe = describe;
  }

  /**
   * A Shiki highlighter with the NX grammar and both of `themes` loaded, to color with. Without one
   * the element loads its own, shared by every element on the page. Not one behind a Monaco editor:
   * coloring switches the highlighter's current theme, which `@shikijs/monaco` tokenizes with.
   */
  get highlighter(): NxValueHighlighter | undefined {
    return this.#highlighter;
  }

  set highlighter(highlighter: NxValueHighlighter | undefined) {
    this.#highlighter = highlighter;
    this.#render();
  }

  /** The light and dark Shiki themes to color with. Default `github-light` and `github-dark`. */
  get themes(): NxValueThemes {
    return this.#themes;
  }

  set themes(themes: NxValueThemes) {
    this.#themes = themes;
    this.#loadedHighlighter = undefined;
    this.#render();
  }

  attributeChangedCallback(name: string): void {
    if (name === "truncated") {
      this.#notice.hidden = !this.truncated;
    } else if (name === "stale") {
      this.#badge.hidden = !this.stale;
    }
  }

  /** Folds or unfolds the node at `index`, if it can fold. */
  setFolded(index: number, folded: boolean): void {
    const element = this.#rendered?.elements[index];
    if (element === undefined || !element.classList.contains("foldable")) {
      return;
    }
    const toggle = element.querySelector<HTMLButtonElement>(":scope > .fold")!;
    const full = element.querySelector<HTMLElement>(":scope > .full")!;
    const summary = element.querySelector<HTMLElement>(":scope > .folded")!;
    toggle.setAttribute("aria-expanded", String(!folded));
    toggle.setAttribute("aria-label", `${folded ? "Unfold" : "Fold"} ${toggle.dataset["label"] ?? ""}`);
    full.hidden = folded;
    summary.hidden = !folded;
  }

  /** Whether the node at `index` is folded. */
  isFolded(index: number): boolean {
    const element = this.#rendered?.elements[index];
    return element?.querySelector<HTMLElement>(":scope > .full")?.hidden === true;
  }

  /**
   * Renders the value. Recoloring the same value, once a highlighter arrives, keeps what the
   * reader folded and what they are pointing at; a new value starts afresh.
   */
  #render(sameValue = false): void {
    const folded = sameValue
      ? (this.#rendered?.elements ?? []).flatMap((_, index) => (this.isFolded(index) ? [index] : []))
      : [];
    const hovered = sameValue ? this.#hovered : undefined;
    this.#setHovered(undefined);
    const value = this.#value;
    this.#pre.replaceChildren();
    this.#rendered = undefined;
    this.#copy.hidden = value === undefined;
    if (value === undefined) {
      return;
    }

    this.#loadedHighlighter ??= loadedHighlighter(this.#themes);
    const highlighter = this.#highlighter ?? this.#loadedHighlighter;
    let colors: readonly ColoredRange[] = [];
    if (value.text.length > MAX_COLORED_CHARACTERS) {
      // Too long to color without holding up the page; shown as plain text.
    } else if (highlighter !== undefined) {
      colors = colorize(highlighter, value.text, this.#themes);
    } else {
      this.#load(value);
    }
    this.#rendered = renderValue(this.ownerDocument, value, colors);
    this.#pre.append(this.#rendered.root);
    for (const index of folded) {
      this.setFolded(index, true);
    }
    if (hovered !== undefined) {
      this.#setHovered(hovered);
    }
  }

  #load(value: NxValueText): void {
    const themes = this.#themes;
    loadHighlighter(themes).then(
      (highlighter) => {
        if (this.#themes !== themes) {
          return;
        }
        this.#loadedHighlighter = highlighter;
        // Shown uncolored meanwhile; color it now unless it has been replaced.
        if (this.#value === value && this.#highlighter === undefined) {
          this.#render(true);
        }
      },
      () => {
        // Without a highlighter the value stays readable, uncolored.
      }
    );
  }

  #nodeIndexAt(target: EventTarget | null): number | undefined {
    const element = (target as Element | null)?.closest?.(".node");
    if (element === null || element === undefined || !this.#pre.contains(element)) {
      return undefined;
    }
    const index = Number((element as HTMLElement).dataset["node"]);
    return Number.isInteger(index) ? index : undefined;
  }

  #onPointerMove(event: PointerEvent): void {
    this.#setHovered(this.#nodeIndexAt(event.target));
  }

  #onFocusIn(event: FocusEvent): void {
    const index = this.#nodeIndexAt(event.target);
    if (index !== undefined) {
      this.#setHovered(index, true);
    }
  }

  #onClick(event: MouseEvent): void {
    const target = event.target as Element;
    const button = target.closest?.("button");
    if (button !== null && button !== undefined) {
      const index = this.#nodeIndexAt(button);
      if (index !== undefined) {
        this.setFolded(index, button.classList.contains("fold") ? !this.isFolded(index) : false);
      }
      return;
    }
    // A drag that selected text is a selection, not a choice.
    const selection = this.shadowRoot?.ownerDocument.getSelection?.();
    if (selection !== null && selection !== undefined && !selection.isCollapsed) {
      return;
    }
    const index = this.#nodeIndexAt(target);
    if (index !== undefined) {
      this.#navigate(index);
    }
  }

  #onKeyDown(event: KeyboardEvent): void {
    const elements = this.#visibleNodeElements();
    const focused = this.#nodeIndexAt(this.shadowRoot?.activeElement ?? null);
    const position = focused === undefined ? -1 : elements.findIndex((element) => Number(element.dataset["node"]) === focused);

    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const next =
          event.key === "ArrowDown"
            ? Math.min(position + 1, elements.length - 1)
            : Math.max(position - 1, 0);
        elements[next]?.focus();
        break;
      }
      case "Enter":
        if (focused !== undefined) {
          event.preventDefault();
          this.#navigate(focused);
        }
        break;
      case " ":
        if (focused !== undefined && this.#rendered?.elements[focused]?.classList.contains("foldable")) {
          event.preventDefault();
          this.setFolded(focused, !this.isFolded(focused));
        }
        break;
      case "Escape":
        this.#setHovered(undefined);
        break;
    }
  }

  /** Node elements in text order, leaving out those inside a folded node. */
  #visibleNodeElements(): HTMLElement[] {
    return Array.from(this.#pre.querySelectorAll<HTMLElement>(".node")).filter(
      (element) => element.closest("[hidden]") === null
    );
  }

  #navigate(index: number): void {
    const node = this.#value?.nodes[index];
    if (node?.declaration === undefined) {
      return;
    }
    const detail: NxValueNavigateDetail = { node, index, declaration: node.declaration };
    this.dispatchEvent(
      new CustomEvent(NX_VALUE_NAVIGATE_EVENT, { detail, bubbles: true, composed: true })
    );
  }

  #setHovered(index: number | undefined, immediately = false): void {
    if (index === this.#hovered) {
      return;
    }
    const elements = this.#rendered?.elements;
    if (this.#hovered !== undefined) {
      elements?.[this.#hovered]?.classList.remove("hovered");
    }
    this.#hovered = index;
    clearTimeout(this.#hoverTimer);
    this.#hoverRequest += 1;
    this.#hover.hidden = true;
    if (index === undefined) {
      return;
    }
    elements?.[index]?.classList.add("hovered");
    if (immediately) {
      void this.#showHover(index);
    } else {
      this.#hoverTimer = setTimeout(() => void this.#showHover(index), hoverDelay);
    }
  }

  async #showHover(index: number): Promise<void> {
    const value = this.#value;
    const node = value?.nodes[index];
    if (value === undefined || node === undefined) {
      return;
    }
    const request = ++this.#hoverRequest;
    let markdown: string | undefined;
    try {
      markdown = await this.#describe?.(node, index, value);
    } catch {
      markdown = undefined;
    }
    if (request !== this.#hoverRequest || this.#hovered !== index) {
      return;
    }
    const highlighter = this.#highlighter ?? this.#loadedHighlighter;
    this.#hover.replaceChildren(
      renderMarkdown(
        this.ownerDocument,
        markdown ?? defaultDescription(value, index),
        highlighter,
        this.#themes
      )
    );
    this.#hover.hidden = false;
    this.#positionHover(index);
  }

  #positionHover(index: number): void {
    const element = this.#rendered?.elements[index];
    if (element === undefined) {
      return;
    }
    const frame = this.#frame.getBoundingClientRect();
    const line = element.getClientRects()[0] ?? element.getBoundingClientRect();
    const top = line.bottom - frame.top + 2;
    const left = Math.max(0, Math.min(line.left - frame.left, frame.width - this.#hover.offsetWidth));
    this.#hover.style.top = `${top}px`;
    this.#hover.style.left = `${left}px`;
  }

  async #copyText(): Promise<void> {
    const value = this.#value;
    if (value === undefined) {
      return;
    }
    let copied = false;
    try {
      await navigator.clipboard.writeText(value.text);
      copied = true;
    } catch {
      copied = false;
    }
    this.#copy.textContent = copied ? "Copied" : "Copy failed";
    clearTimeout(this.#copyTimer);
    this.#copyTimer = setTimeout(() => {
      this.#copy.textContent = "Copy";
    }, 1500);
  }
}

/**
 * Defines `<nx-value>` in `registry`, the page's by default. Importing this module already does it
 * where there is a DOM; calling it again is harmless.
 */
export function defineNxValueElement(
  registry: CustomElementRegistry | undefined = globalThis.customElements
): void {
  if (registry !== undefined && registry.get(NX_VALUE_TAG) === undefined) {
    registry.define(NX_VALUE_TAG, NxValueElement);
  }
}

defineNxValueElement();

declare global {
  interface HTMLElementTagNameMap {
    "nx-value": NxValueElement;
  }
}
