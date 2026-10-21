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
import type { EditorRange, SourceEdit, SourceRole, SourceTree } from "@nx-lang/language-protocol";

import { cardOf } from "./card.js";
import { editOf, editTargets, isHexColor, isNumericLiteral, isWholeValue, removalOf, runProblem, type EditTarget } from "./edit.js";
import { caseOptions, pickerColor, showValue, stepNumber, valueHost } from "./editors.js";
import { renderMarkdown } from "./markdown.js";
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

/** The event fired when a reader commits an edit, when the host has set `editable`. */
export const NX_EDIT_EVENT = "nx-edit";

/**
 * The event fired when a reader asks to open a value the reading cannot edit, as an expression, in
 * the host's source editor, when the host has set `editable`.
 */
export const NX_OPEN_EVENT = "nx-open";

/** What the `nx-edit` event carries: the target node's key and the edit, for the `sourceEdit` query. */
export interface NxEditDetail {
  /** The key of the node the edit is aimed at: a literal, a case, a run of text or an attribute. */
  readonly key: string;
  readonly edit: SourceEdit;
}

/** What the `nx-open` event carries: the node to open in the source. */
export interface NxOpenDetail {
  readonly key: string;
  readonly role: SourceRole;
  readonly range: EditorRange;
}

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

/** Renders a doc comment's Markdown as the host wants it read. */
export type NxViewerMarkdown = (markdown: string) => Node;

/** The element's properties, which a host may set before the element is defined. */
const PROPERTIES = ["tree", "text", "colors", "icons", "markdown", "folded", "selection", "stale", "editable"] as const;

/** How far the pointer moves sideways, in pixels, for a number's name dragged to step it once. */
const SCRUB_STEP = 2;

/** How far the pointer moves before a press on a number's name becomes a drag, in pixels. */
const SCRUB_SLOP = 3;

/** How long the pointer rests on a name or a value before its card opens, in milliseconds. */
const CARD_DELAY = 500;

/** How long a card stays once the pointer leaves it and what it describes, to reach it. */
const CARD_GRACE = 300;

/** The room between a card and what it describes, in pixels. */
const CARD_GAP = 4;

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

/** An editor open on one value: a field in the value's place, or a popover over the reading. */
interface EditSession {
  readonly target: EditTarget;
  /** The value's rendering. */
  readonly span: HTMLElement;
  /** Where the value's text sits, and what it held, put back when the edit is cancelled. */
  readonly host?: HTMLElement;
  readonly restore?: readonly Node[];
  /** A text, number or run field. */
  readonly field?: HTMLElement;
  /** A popover's field: a case's filter or a color's hex. */
  readonly input?: HTMLInputElement;
  /** A case popover's options, and the one Enter chooses. */
  readonly options?: readonly HTMLElement[];
  active?: HTMLElement | undefined;
  closed: boolean;
}

/** An editor open when the reading was drawn again, to open again on the same value. */
interface Reopen {
  readonly key: string;
  /** What the editor held. */
  readonly draft: string | undefined;
  /** Where the caret and selection were in it, in UTF-16 offsets. */
  readonly caret: Caret | undefined;
}

interface Caret {
  readonly start: number;
  readonly end: number;
}

/** A number's name being dragged sideways to change it. */
interface Scrub {
  target: EditTarget;
  name: HTMLElement;
  readonly pointer: number;
  readonly x: number;
  /** The number when the drag began. */
  readonly from: string;
  /** The number now, once the press became a drag. */
  value: string | undefined;
}

function keyOf(element: Element): string | undefined {
  return (element as HTMLElement).dataset["key"];
}

/**
 * The `<nx-viewer>` element.
 *
 * <para>The attribute `stale` mirrors the property of the same name. Everything else is a property:
 * `tree`, `text`, `colors`, `icons`, `markdown`, `folded`, `selection` and `editable`.</para>
 */
export class NxViewerElement extends ElementBase {
  static observedAttributes = ["stale"];

  #tree: SourceTree | undefined;
  #text = "";
  #colors: readonly NxViewerColor[] = [];
  #icons: NxViewerIcons | undefined;
  #markdown: NxViewerMarkdown | undefined;
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
  /**
   * The key of the name or value under the pointer, whose card opens once the pointer rests there.
   * A node marked on several spans, as a line's name and its `=`, is one target.
   */
  #pointedKey: string | undefined;
  /** The key of the node the open card describes. */
  #cardKey: string | undefined;
  /** What scrolls the reading while a card is open, which closes the card when it scrolls. */
  #cardScroller: Element | undefined;
  /** Whether the open card was asked for from the keyboard, which the pointer does not close. */
  #cardByKeyboard = false;
  #cardOpenTimer: ReturnType<typeof setTimeout> | undefined;
  #cardCloseTimer: ReturnType<typeof setTimeout> | undefined;
  /** Whether the reader may edit values in place. */
  #editable = false;
  /** The values of the tree shown that the reading can edit, by key, while it may. */
  #targets = new Map<string, EditTarget>();
  /** The editor open, if any. */
  #session: EditSession | undefined;
  /** The editor to open again once the reading is drawn with a tree. */
  #reopen: Reopen | undefined;
  /** Each value edited since the host last set a tree, as the reader left it, shown until it does. */
  #pending = new Map<string, string>();
  /** The attributes removed since the host last set a tree, shown muted until it does. */
  #removed = new Set<string>();
  #scrub: Scrub | undefined;
  /** Set when a drag ends, so the click the press makes does not also select. */
  #ignoreClick = false;

  readonly #badge: HTMLSpanElement;
  readonly #body: HTMLDivElement;
  /** The highlights of the selected and the hovered unit, drawn behind the text. */
  readonly #highlights: HTMLDivElement;
  /** Says what the arrow keys selected, for a screen reader, since focus stays on the body. */
  readonly #announcer: HTMLDivElement;
  readonly #frame: HTMLDivElement;
  /** The hover card: what a name or a value under the pointer is, and its doc. */
  readonly #card: HTMLDivElement;
  /** The popover a case or a color is chosen in. */
  readonly #popover: HTMLDivElement;
  /** The quiet button at the end of a selected row whose property may be left out. */
  readonly #rowAction: HTMLSpanElement;
  /** Closes the card when anything around the reading scrolls, or Escape is pressed anywhere. */
  readonly #onDocumentScroll = (): void => this.#closeCard();
  readonly #onDocumentKey = (event: KeyboardEvent): void => {
    if (event.key === "Escape") {
      this.#closeCard();
    }
  };
  /** Cancels a popover when the pointer goes down anywhere outside it. */
  readonly #onDocumentPointerDown = (event: PointerEvent): void => {
    const session = this.#session;
    if (session !== undefined && session.field === undefined && !event.composedPath().includes(this.#popover)) {
      this.#cancel(session, false);
    }
  };

  constructor() {
    super();
    const shadow = this.attachShadow({ mode: "open" });
    const document = this.ownerDocument;

    const style = document.createElement("style");
    style.textContent = styles;

    const frame = document.createElement("div");
    frame.className = "frame";
    this.#frame = frame;

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

    this.#card = document.createElement("div");
    this.#card.className = "card";
    this.#card.setAttribute("role", "tooltip");
    this.#card.hidden = true;
    // The pointer may cross onto the card, to select its text or follow its links.
    this.#card.addEventListener("pointerenter", () => clearTimeout(this.#cardCloseTimer));
    this.#card.addEventListener("pointerleave", () => {
      if (!this.#cardByKeyboard) {
        this.#closeCardSoon();
      }
    });

    this.#popover = document.createElement("div");
    this.#popover.className = "popover";
    this.#popover.hidden = true;
    this.#popover.addEventListener("keydown", (event) => this.#onPopoverKey(event));
    // Choosing with the pointer keeps the focus in the popover's field.
    this.#popover.addEventListener("pointerdown", (event) => {
      if (!(event.target instanceof HTMLInputElement)) {
        event.preventDefault();
      }
    });
    this.#popover.addEventListener("click", (event) => {
      const option = (event.target as Element).closest?.(".option") as HTMLElement | null;
      const session = this.#session;
      if (option !== null && session !== undefined && session.options !== undefined) {
        this.#choose(session, option.dataset["name"]!);
      }
    });
    this.#popover.addEventListener("focusout", (event) => {
      const next = event.relatedTarget as Node | null;
      const session = this.#session;
      if (session !== undefined && session.field === undefined && next !== null && !this.#popover.contains(next)) {
        this.#cancel(session, false);
      }
    });

    this.#rowAction = document.createElement("span");
    this.#rowAction.className = "row-action";
    this.#rowAction.setAttribute("role", "button");
    this.#rowAction.hidden = true;
    this.#rowAction.addEventListener("pointerdown", (event) => event.preventDefault());

    frame.append(toolbar, this.#body, this.#announcer, this.#card, this.#popover);
    shadow.append(style, frame);

    this.#body.addEventListener("click", (event) => this.#onClick(event));
    this.#body.addEventListener("dblclick", (event) => this.#onDoubleClick(event));
    this.#body.addEventListener("pointerdown", (event) => this.#onPointerDown(event));
    this.#body.addEventListener("pointermove", (event) => this.#onPointerMove(event));
    this.#body.addEventListener("pointerup", (event) => this.#onPointerUp(event, true));
    this.#body.addEventListener("pointercancel", (event) => this.#onPointerUp(event, false));
    // A focus ring is for a reader on the keyboard; after a click, the selection says where they are.
    this.#body.addEventListener("pointerdown", () => {
      this.#body.classList.add("by-pointer");
      this.#closeCard();
    });
    frame.addEventListener("keydown", () => this.#body.classList.remove("by-pointer"));
    frame.addEventListener("keydown", (event) => this.#onKeyDown(event));
    this.#body.addEventListener("pointerover", (event) => {
      this.#setHoveredUnit(this.#unitOf(event.target as Element));
      this.#point(this.#hoverTargetOf(event.target));
    });
    this.#body.addEventListener("pointerleave", () => {
      this.#setHoveredUnit(undefined);
      this.#point(undefined);
    });
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

  connectedCallback(): void {
    this.ownerDocument.addEventListener("scroll", this.#onDocumentScroll, true);
    this.ownerDocument.addEventListener("keydown", this.#onDocumentKey);
    this.ownerDocument.addEventListener("pointerdown", this.#onDocumentPointerDown, true);
  }

  disconnectedCallback(): void {
    this.ownerDocument.removeEventListener("scroll", this.#onDocumentScroll, true);
    this.ownerDocument.removeEventListener("keydown", this.#onDocumentKey);
    this.ownerDocument.removeEventListener("pointerdown", this.#onDocumentPointerDown, true);
    this.#closeCard();
    this.#dropSession();
  }

  /** The source tree to show, as the language service's `sourceTree` query answers it. */
  get tree(): SourceTree | undefined {
    return this.#tree;
  }

  set tree(tree: SourceTree | undefined) {
    this.#tree = tree;
    // A new tree says what each edit made of the source, or that it made nothing.
    this.#pending.clear();
    this.#removed.clear();
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
   * Renders a doc comment's Markdown for the hover cards, as the host's other views render it. The
   * host answers for what it returns. Without it, a built-in renderer reads paragraphs, code,
   * emphasis, lists and links to the web, and anything else, raw HTML included, as text.
   */
  get markdown(): NxViewerMarkdown | undefined {
    return this.#markdown;
  }

  set markdown(markdown: NxViewerMarkdown | undefined) {
    this.#markdown = typeof markdown === "function" ? markdown : undefined;
    this.#closeCard();
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
   * Whether the reader may edit values where they read: a string, a number, a boolean, a union's
   * case, a color or a line of text. Each edit fires `nx-edit` for the host to make in its source,
   * and a value the reading cannot edit, opened with Enter or a double-click, fires `nx-open`. The
   * element never changes the text or the tree itself. Off by default, when the reading reads as it
   * does without it.
   */
  get editable(): boolean {
    return this.#editable;
  }

  set editable(editable: boolean) {
    const next = editable === true;
    if (next !== this.#editable) {
      this.#editable = next;
      this.#render();
    }
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
    // What the card described may be gone, and so may what the pointer was over. An editor open is
    // opened again on its value, as the reader left it, when the value is still there.
    this.#closeCard();
    // A host that clears the tree before it sets the next one gets the editor back with that one.
    const session = this.#session;
    if (session !== undefined && !session.closed) {
      this.#reopen = { key: session.target.key, draft: this.#draftOf(session), caret: this.#caretOf(session) };
    }
    this.#dropSession();
    this.#pointedKey = undefined;
    this.#rendered.clear();
    this.#hoveredUnit = undefined;
    this.#body.replaceChildren();
    this.#body.classList.toggle("editable", this.#editable);
    this.#index = undefined;
    this.#targets = new Map();
    if (tree === undefined) {
      return;
    }
    // A drag on a number's name goes on with the next tree, on the same number.
    const scrub = this.#scrub;
    this.#scrub = undefined;
    this.#body.classList.remove("scrubbing");
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
    // A property's element value on a line or a header that does not fit is measured once that one
    // has its rows, so each pass settles one level of values, and values nested deeper than this
    // keep their innermost lines.
    for (let pass = 0; pass < 6 && this.#fitWidth > 0; pass++) {
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
    if (this.#editable) {
      this.#targets = editTargets(index);
      this.#markEditable();
      if (scrub !== undefined) {
        this.#resumeScrub(scrub);
      }
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
    this.#body.append(this.#highlights, this.#rowAction);
    this.#paintHighlights();
    this.#turnChevron();
    const reopen = this.#reopen;
    this.#reopen = undefined;
    if (reopen !== undefined && this.#targets.has(reopen.key) && this.#rendered.has(reopen.key)) {
      this.#openEditor(reopen.key, reopen.draft, reopen.caret);
    }
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
   * or, for a folded group, whose header runs past its line, since it is cut rather than wrapped;
   * of those, the outermost only.
   */
  #wrapped(): string[] {
    const source = this.#body.querySelector<HTMLElement>(".source");
    if (source === null) {
      return [];
    }
    const lineHeight = parseFloat(getComputedStyle(source).lineHeight) || 18;
    // Each element found, and what of it was measured: its line, or its group's header.
    const found: { element: HTMLElement; target: HTMLElement }[] = [];
    for (const element of this.#body.querySelectorAll<HTMLElement>(".fit-check")) {
      const target = element.classList.contains("sheet") ? element.querySelector<HTMLElement>(":scope > .sheet-head") : element;
      if (target === null) {
        continue;
      }
      if (element.classList.contains("folded")) {
        if (target.scrollWidth > target.clientWidth + 1) {
          found.push({ element, target });
        }
        continue;
      }
      // An inline line has a box to each line it falls on; a block has one, as tall as its lines.
      const rects = [...target.getClientRects()];
      const lines = rects.length > 1 ? new Set(rects.map((rect) => Math.round(rect.top))).size : (rects[0]?.height ?? 0) / lineHeight;
      if (lines > 1.5) {
        found.push({ element, target });
      }
    }
    // An element on the line or the header of one that does not fit is not where it will be once
    // that one has its rows, as a property's value moved to a row: it is measured again on the next
    // pass. What a group holds under its header is where it will be, and counts now.
    return found
      .filter(({ element }) => !found.some((outer) => outer.element !== element && outer.target.contains(element)))
      .map(({ element }) => element.dataset["key"]!);
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
    this.#placeRowAction();
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

  /**
   * Shows the quiet button that leaves out the selected row's property, at the end of the row's first
   * line, when the reader may edit and the property has a default or is optional.
   */
  #placeRowAction(): void {
    const action = this.#rowAction;
    const unit = this.#selectedUnit();
    const at = unit === undefined || !unit.matches(".sheet-row") ? undefined : this.#index?.byKey.get(keyOf(unit) ?? "");
    const removal = !this.#editable || at === undefined || this.#removed.has(keyOf(unit!)!) ? undefined : removalOf(this.#index!, at);
    if (removal === undefined || this.#session?.span !== undefined && unit!.contains(this.#session.span)) {
      action.hidden = true;
      return;
    }
    const label = removal === "reset" ? "Reset to default" : "Clear";
    action.textContent = label;
    action.title = label;
    action.setAttribute("aria-label", label);
    action.hidden = false;
    const frame = this.#body.getBoundingClientRect();
    const first = unit!.querySelector<HTMLElement>(":scope > .sheet-name")?.getBoundingClientRect() ?? unit!.getBoundingClientRect();
    action.style.top = `${first.top - frame.top + this.#body.scrollTop}px`;
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
    // A card asked for from the keyboard is the selected unit's.
    if (this.#cardByKeyboard) {
      this.#closeCard();
    }
    for (const element of this.#body.querySelectorAll(".selected")) {
      element.classList.remove("selected");
      element.removeAttribute("aria-current");
    }
    queueMicrotask(() => this.#paintHighlights());
    const index = this.#index;
    const at = key === undefined ? undefined : index?.byKey.get(key);
    if (index === undefined || key === undefined || at === undefined) {
      this.#selection = undefined;
      this.#markUnitValues();
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
    this.#markUnitValues();
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

  // ---- Editing ----------------------------------------------------------------------------

  /**
   * Marks each value the reading can edit, and the name of each number's property, which drags
   * sideways to change it; then shows the values edited since the host last set a tree.
   */
  #markEditable(): void {
    const index = this.#index!;
    const names = new Map<string, HTMLElement[]>();
    for (const name of this.#body.querySelectorAll<HTMLElement>(".sheet-name[data-hover], .line-name[data-hover]")) {
      const list = names.get(name.dataset["hover"]!) ?? [];
      list.push(name);
      names.set(name.dataset["hover"]!, list);
    }
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      const target = this.#targets.get(element.dataset["key"]!);
      if (target === undefined) {
        continue;
      }
      element.classList.add("editable-value", `edit-${target.kind}`);
      const owner = index.tree.nodes[target.at]!.parent;
      if (target.kind === "number" && owner !== undefined && index.tree.nodes[owner]!.role === "attribute") {
        for (const name of names.get(index.tree.nodes[owner]!.key) ?? []) {
          name.classList.add("scrub");
          name.dataset["scrubKey"] = target.key;
        }
      }
    }
    for (const [key, value] of this.#pending) {
      const target = this.#targets.get(key);
      const span = this.#rendered.get(key);
      if (target !== undefined && span !== undefined) {
        showValue(span, target, this.#writtenOf(target), value);
      }
    }
    for (const key of this.#removed) {
      this.#rendered.get(key)?.classList.add("pending");
    }
  }

  /** The selected unit, as it is drawn, when the selection is one. */
  #selectedUnit(): HTMLElement | undefined {
    const drawn = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    return drawn?.matches(DRAWN_UNITS) === true ? drawn : undefined;
  }

  /** The editable values `unit` shows as its own, not those of a unit inside it, in reading order. */
  #unitValues(unit: HTMLElement): HTMLElement[] {
    const found: HTMLElement[] = [];
    for (const value of unit.querySelectorAll<HTMLElement>(".editable-value")) {
      if (this.#unitOf(value) === unit && !found.some((other) => keyOf(other) === keyOf(value))) {
        found.push(value);
      }
    }
    return found;
  }

  /** Marks the values of the selected unit, which a click opens and the pointer marks over. */
  #markUnitValues(): void {
    for (const value of this.#body.querySelectorAll(".in-unit")) {
      value.classList.remove("in-unit");
    }
    if (!this.#editable) {
      return;
    }
    const unit = this.#selectedUnit();
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const values = unit !== undefined ? this.#unitValues(unit) : selected?.matches(".editable-value") === true ? [selected] : [];
    values.forEach((value) => value.classList.add("in-unit"));
  }

  /** The text of the value `target` names as the tree writes it. */
  #writtenOf(target: EditTarget): string {
    return this.#index?.tree.nodes[target.at]?.value ?? "";
  }

  /** The value `target` reads as now: as the reader left it, or as the tree gives it. */
  #currentOf(target: EditTarget): string {
    return this.#pending.get(target.key) ?? target.value;
  }

  #fire(name: string, detail: NxEditDetail | NxOpenDetail): void {
    this.dispatchEvent(new CustomEvent(name, { detail, bubbles: true, composed: true }));
  }

  /**
   * Opens the editor of the value with `key`, holding `draft` with `caret` when they are given, as
   * when a reading drawn again opens an editor the reader had open; a boolean toggles instead.
   */
  #openEditor(key: string, draft?: string, caret?: Caret): void {
    const target = this.#targets.get(key);
    const span = this.#rendered.get(key);
    if (target === undefined || span === undefined) {
      return;
    }
    if (this.#session !== undefined) {
      this.#cancel(this.#session, false);
    }
    this.#closeCard();
    const current = this.#currentOf(target);
    switch (target.kind) {
      case "boolean":
        this.#commitValue(target, current === "true" ? "false" : "true");
        return;
      case "case":
        this.#openCases(target, span, draft ?? "", caret);
        return;
      case "color":
        this.#openColor(target, span, draft ?? current, caret);
        return;
      default:
        this.#openField(target, span, draft ?? current, caret);
    }
  }

  /** Where the caret and selection are in the open editor, or nothing when it has no focus. */
  #caretOf(session: EditSession): Caret | undefined {
    const input = session.input;
    if (input !== undefined) {
      return input.selectionStart === null ? undefined : { start: input.selectionStart, end: input.selectionEnd ?? input.selectionStart };
    }
    const field = session.field;
    const selection = this.#domSelection();
    if (field === undefined || selection === null || selection === undefined || selection.rangeCount === 0) {
      return undefined;
    }
    const range = selection.getRangeAt(0);
    if (!field.contains(range.startContainer) || !field.contains(range.endContainer)) {
      return undefined;
    }
    const offset = (node: Node, at: number): number => {
      const before = this.ownerDocument.createRange();
      before.selectNodeContents(field);
      before.setEnd(node, at);
      return before.toString().length;
    };
    return { start: offset(range.startContainer, range.startOffset), end: offset(range.endContainer, range.endOffset) };
  }

  /** Puts the caret and selection back in `field`, as `caret` says, within what it holds. */
  #placeCaret(field: HTMLElement, caret: Caret): void {
    const text = field.firstChild;
    const length = text?.textContent?.length ?? 0;
    const selection = this.#domSelection();
    try {
      const range = this.ownerDocument.createRange();
      if (text === null) {
        range.setStart(field, 0);
        range.collapse(true);
      } else {
        range.setStart(text, Math.min(caret.start, length));
        range.setEnd(text, Math.min(Math.max(caret.end, caret.start), length));
      }
      selection?.removeAllRanges();
      selection?.addRange(range);
    } catch {
      // A selection the DOM cannot make leaves the caret where it is.
    }
  }

  /** The selection of the reading's shadow root, where the browser keeps one apart, or the document's. */
  #domSelection(): Selection | null | undefined {
    const root = this.shadowRoot as (ShadowRoot & { getSelection?(): Selection | null }) | null;
    return root?.getSelection?.() ?? this.ownerDocument.getSelection?.();
  }

  /** What an open editor holds now, to open it again with. */
  #draftOf(session: EditSession): string | undefined {
    return session.field !== undefined ? (session.field.textContent ?? "") : session.input?.value;
  }

  /** A field in the value's own place, holding `text`, that grows with it and wraps in the reading. */
  #openField(target: EditTarget, span: HTMLElement, text: string, caret?: Caret): void {
    const { host, lead, trail, keep } = valueHost(span, this.#writtenOf(target));
    const restore = [...host.childNodes];
    const field = this.ownerDocument.createElement("span");
    field.className = `field field-${target.kind}`;
    field.setAttribute("contenteditable", "plaintext-only");
    field.setAttribute("role", "textbox");
    field.setAttribute("aria-label", this.#labelOf(target));
    field.spellcheck = target.kind === "run";
    field.textContent = text;
    host.replaceChildren(...keep, ...(lead === undefined ? [] : [lead]), field, ...(trail === undefined ? [] : [trail]));
    span.classList.add("editing");
    const session: EditSession = { target, span, host, restore, field, closed: false };
    this.#session = session;
    field.addEventListener("keydown", (event) => this.#onFieldKey(event, session));
    field.addEventListener("input", () => this.#validate(session));
    field.addEventListener("blur", () => {
      if (!session.closed) {
        this.#finishField(session, "blur");
      }
    });
    this.#validate(session);
    this.#paintHighlights();
    field.focus({ preventScroll: true });
    // A field opened afresh is selected whole, so typing replaces it; one opened again keeps the caret.
    if (caret === undefined) {
      this.#selectAll(field);
    } else {
      this.#placeCaret(field, caret);
    }
  }

  /** Selects all `field` holds, so typing replaces it. */
  #selectAll(field: HTMLElement): void {
    const selection = this.#domSelection();
    try {
      selection?.selectAllChildren(field);
    } catch {
      // A selection the DOM cannot make leaves the caret where it is.
    }
  }

  /** What the value is called for a reader who cannot see it: its property's name, or its kind. */
  #labelOf(target: EditTarget): string {
    const index = this.#index!;
    const owner = index.tree.nodes[target.at]!.parent;
    const name = owner === undefined ? undefined : index.tree.nodes[owner]!.name;
    return name ?? (target.kind === "run" ? "Text" : "Value");
  }

  /** Why the field's entry cannot be taken, or nothing when it can. */
  #problemOf(target: EditTarget, value: string): string | undefined {
    switch (target.kind) {
      case "number":
        if (!isNumericLiteral(value.trim())) {
          return "Not a number";
        }
        return target.signed === false && value.trim().startsWith("-") ? "Cannot be negative here" : undefined;
      case "run":
        return runProblem(value);
      case "color":
        return isHexColor(value.trim()) ? undefined : "Not a hex color";
      default:
        return undefined;
    }
  }

  /** Marks the editor's entry as invalid while the editor cannot take it. */
  #validate(session: EditSession): void {
    const input = session.field ?? session.input;
    if (input === undefined || session.target.kind === "case") {
      return;
    }
    const problem = this.#problemOf(session.target, this.#draftOf(session) ?? "");
    input.classList.toggle("invalid", problem !== undefined);
    input.setAttribute("aria-invalid", String(problem !== undefined));
    if (problem === undefined) {
      input.removeAttribute("title");
    } else {
      input.title = problem;
    }
  }

  #onFieldKey(event: KeyboardEvent, session: EditSession): void {
    // A field's keys are its own, Ctrl+Z and Ctrl+Y included, not the reading's or the host's.
    event.stopPropagation();
    const field = session.field!;
    switch (event.key) {
      case "Enter":
        event.preventDefault();
        this.#finishField(session, "key");
        return;
      case "Escape":
        event.preventDefault();
        this.#cancel(session, true);
        return;
      case "Tab":
        event.preventDefault();
        if (this.#finishField(session, "key")) {
          this.#openNext(session.target.key, event.shiftKey ? -1 : 1);
        }
        return;
      case "ArrowUp":
      case "ArrowDown": {
        if (session.target.kind !== "number") {
          return;
        }
        event.preventDefault();
        const steps = (event.key === "ArrowUp" ? 1 : -1) * (event.shiftKey ? 10 : 1);
        const stepped = stepNumber(field.textContent ?? "", steps, session.target.signed !== false);
        if (stepped !== undefined) {
          field.textContent = stepped;
          this.#validate(session);
          this.#selectAll(field);
        }
        return;
      }
    }
  }

  /**
   * Commits the field's entry, or, when it cannot be taken, keeps the field open and marked; leaving
   * the field with such an entry drops it. Answers whether the field closed.
   */
  #finishField(session: EditSession, how: "key" | "blur"): boolean {
    const target = session.target;
    let value = session.field!.textContent ?? "";
    if (target.kind === "number") {
      value = value.trim();
    }
    if (this.#problemOf(target, value) !== undefined) {
      if (how === "blur") {
        this.#cancel(session, false);
        return true;
      }
      this.#validate(session);
      return false;
    }
    this.#endSession(session, how === "key");
    if (value === this.#currentOf(target)) {
      session.host!.replaceChildren(...session.restore!);
    } else {
      this.#commitValue(target, value);
    }
    return true;
  }

  /** Closes `session` without an edit, putting back what its value showed. */
  #cancel(session: EditSession, refocus: boolean): void {
    this.#endSession(session, refocus);
    session.host?.replaceChildren(...session.restore!);
  }

  /** Closes `session`: its popover goes, and with `refocus`, the reading takes the keys again. */
  #endSession(session: EditSession, refocus: boolean): void {
    session.closed = true;
    if (this.#session === session) {
      this.#session = undefined;
    }
    session.span.classList.remove("editing");
    if (session.field === undefined) {
      this.#popover.hidden = true;
      this.#popover.replaceChildren();
    }
    if (refocus) {
      this.#body.focus({ preventScroll: true });
    }
    this.#paintHighlights();
  }

  /** Forgets the open editor, as when the reading is drawn again. */
  #dropSession(): void {
    const session = this.#session;
    if (session !== undefined) {
      session.closed = true;
      this.#session = undefined;
    }
    this.#popover.hidden = true;
    this.#popover.replaceChildren();
  }

  /** Sets `target` to `value`: fires `nx-edit` and shows the value, muted, until the host's tree. */
  #commitValue(target: EditTarget, value: string): void {
    if (value === this.#currentOf(target)) {
      return;
    }
    this.#pending.set(target.key, value);
    const span = this.#rendered.get(target.key);
    if (span !== undefined) {
      showValue(span, target, this.#writtenOf(target), value);
    }
    this.#fire(NX_EDIT_EVENT, { key: target.key, edit: editOf(target, value) });
  }

  /**
   * Opens the editor of the editable value after or before the one with `key`, in reading order,
   * selecting its unit. A boolean, which has no editor, is passed over.
   */
  #openNext(key: string, direction: 1 | -1): void {
    const values: HTMLElement[] = [];
    for (const value of this.#body.querySelectorAll<HTMLElement>(".editable-value")) {
      if (!values.some((other) => keyOf(other) === keyOf(value))) {
        values.push(value);
      }
    }
    const from = values.findIndex((value) => keyOf(value) === key);
    let at = from + direction;
    while (at >= 0 && at < values.length && this.#targets.get(keyOf(values[at]!)!)?.kind === "boolean") {
      at += direction;
    }
    const next = from < 0 ? undefined : values[at];
    if (next === undefined) {
      return;
    }
    const unit = this.#unitOf(next);
    this.#select(keyOf(unit ?? next), true);
    this.#openEditor(keyOf(next)!);
  }

  /**
   * Opens the selected unit's first editable value; or, with `open`, fires `nx-open` for a selected
   * value the reading cannot edit, as a row's expression. Answers whether either happened.
   */
  #openSelected(open: boolean): boolean {
    const unit = this.#selectedUnit();
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const first = unit !== undefined ? this.#unitValues(unit)[0] : selected?.matches(".editable-value") === true ? selected : undefined;
    if (first !== undefined) {
      this.#openEditor(keyOf(first)!);
      return true;
    }
    const index = this.#index;
    const at = this.#selection === undefined ? undefined : index?.byKey.get(this.#selection);
    if (!open || index === undefined || at === undefined) {
      return false;
    }
    const node = index.tree.nodes[at]!;
    // A row opens its value; a unit that is not a value, as a group, opens nothing.
    const value =
      node.role === "attribute"
        ? (index.children[at] ?? []).find((child) => !["comment", "docComment"].includes(index.tree.nodes[child]!.role))
        : unit === undefined
          ? this.#valueAt(at)
          : undefined;
    if (value === undefined) {
      return false;
    }
    this.#fireOpen(value);
    return true;
  }

  /**
   * The value the node at `at` is part of: the nearest node, from it outwards, that is a whole value,
   * and not past an element or a name. Nothing when there is none, as for an element's name.
   */
  #valueAt(at: number): number | undefined {
    const index = this.#index!;
    for (let node: number | undefined = at; node !== undefined; node = index.tree.nodes[node]!.parent) {
      const role = index.tree.nodes[node]!.role;
      if (role === "element" || role === "attribute" || role === "declaration" || role === "sequence") {
        return undefined;
      }
      if (isWholeValue(index, node)) {
        return node;
      }
    }
    return undefined;
  }

  #fireOpen(at: number): void {
    const node = nodeOf(this.#index!, at);
    this.#fire(NX_OPEN_EVENT, { key: node.key, role: node.role, range: node.range });
  }

  /**
   * A double-click on an editable value opens its editor, selecting its unit; on a value the reading
   * cannot edit, it asks the host to open it in the source.
   */
  #onDoubleClick(event: MouseEvent): void {
    if (!this.#editable) {
      return;
    }
    const target = event.target as Element;
    if (this.#session?.field?.contains(target) === true || target.closest?.(".row-action, .fold") != null) {
      return;
    }
    const value = target.closest?.(".editable-value") as HTMLElement | null | undefined;
    if (value !== null && value !== undefined && this.#body.contains(value)) {
      const key = keyOf(value)!;
      // A boolean toggled on the double-click's second click; an editor open is already open.
      if (this.#targets.get(key)?.kind === "boolean" || this.#session?.target.key === key) {
        return;
      }
      const unit = this.#unitOf(value);
      if (keyOf(unit ?? value) !== this.#selection) {
        this.#select(keyOf(unit ?? value), true);
      }
      this.#openEditor(key);
      return;
    }
    const node = target.closest?.("[data-key]") as HTMLElement | null | undefined;
    const at = node === null || node === undefined || !this.#body.contains(node) ? undefined : this.#index?.byKey.get(keyOf(node)!);
    const whole = at === undefined ? undefined : this.#valueAt(at);
    if (whole !== undefined) {
      this.#fireOpen(whole);
    }
  }

  /**
   * Leaves out the selected row's property, when it has a default or is optional, and selects the
   * element it was on. Answers whether it did.
   */
  #removeSelected(): boolean {
    const index = this.#index;
    const key = this.#selection;
    const at = key === undefined ? undefined : index?.byKey.get(key);
    const row = key === undefined ? undefined : this.#rendered.get(key);
    if (!this.#editable || index === undefined || at === undefined || row?.matches(".sheet-row") !== true || this.#removed.has(key!)) {
      return false;
    }
    if (removalOf(index, at) === undefined) {
      return false;
    }
    this.#removed.add(key!);
    row.classList.add("pending");
    this.#fire(NX_EDIT_EVENT, { key: key!, edit: { kind: "remove" } });
    const element = index.tree.nodes[at]!.parent;
    this.#select(element === undefined ? undefined : index.tree.nodes[element]!.key, true);
    return true;
  }

  // ---- Popovers ---------------------------------------------------------------------------

  /** The union's cases with no properties, to choose from, the current one marked, `filter` typed. */
  #openCases(target: EditTarget, span: HTMLElement, filter: string, caret?: Caret): void {
    const document = this.ownerDocument;
    const input = document.createElement("input");
    input.className = "popover-filter";
    input.placeholder = "Filter";
    input.value = filter;
    input.spellcheck = false;
    input.setAttribute("role", "combobox");
    input.setAttribute("aria-label", `${this.#labelOf(target)}: filter cases`);
    input.setAttribute("aria-expanded", "true");
    input.setAttribute("aria-controls", "nx-viewer-options");
    const list = document.createElement("div");
    list.className = "options";
    list.id = "nx-viewer-options";
    list.setAttribute("role", "listbox");
    const current = this.#currentOf(target);
    const options = caseOptions(document, target.cases ?? [], current);
    options.forEach((option, position) => (option.id = `nx-viewer-option-${position}`));
    list.append(...options);
    this.#popover.className = "popover cases";
    this.#popover.replaceChildren(input, list);
    this.#popover.hidden = false;
    span.classList.add("editing");
    const session: EditSession = { target, span, input, options, active: undefined, closed: false };
    this.#session = session;
    input.addEventListener("input", () => this.#filterCases(session));
    this.#filterCases(session, options.find((option) => option.dataset["name"] === current));
    this.#placeCard(span, this.#popover);
    input.focus({ preventScroll: true });
    if (caret !== undefined) {
      input.setSelectionRange(caret.start, caret.end);
    }
  }

  /** Shows the cases whose names hold what the filter holds, and keeps one of them active. */
  #filterCases(session: EditSession, prefer?: HTMLElement): void {
    const query = (session.input?.value ?? "").trim().toLowerCase();
    for (const option of session.options ?? []) {
      option.hidden = query !== "" && !option.dataset["name"]!.toLowerCase().includes(query);
    }
    const shown = (session.options ?? []).filter((option) => !option.hidden);
    const keep = prefer ?? session.active;
    this.#setActive(session, keep !== undefined && shown.includes(keep) ? keep : shown[0]);
  }

  #setActive(session: EditSession, option: HTMLElement | undefined): void {
    session.active = option;
    for (const candidate of session.options ?? []) {
      candidate.classList.toggle("active", candidate === option);
    }
    if (option === undefined) {
      session.input?.removeAttribute("aria-activedescendant");
    } else {
      session.input?.setAttribute("aria-activedescendant", option.id);
      option.scrollIntoView?.({ block: "nearest" });
    }
  }

  /** A hex field and the browser's color picker, for a string its visualizer formats as a color. */
  #openColor(target: EditTarget, span: HTMLElement, text: string, caret?: Caret): void {
    const document = this.ownerDocument;
    const input = document.createElement("input");
    input.className = "popover-hex";
    input.value = text;
    input.spellcheck = false;
    input.setAttribute("aria-label", `${this.#labelOf(target)}: hex color`);
    const picker = document.createElement("input");
    picker.type = "color";
    picker.className = "popover-picker";
    picker.value = pickerColor(text, span.querySelector(".swatch") ?? undefined);
    picker.setAttribute("aria-label", `${this.#labelOf(target)}: pick a color`);
    this.#popover.className = "popover colors";
    this.#popover.replaceChildren(input, picker);
    this.#popover.hidden = false;
    span.classList.add("editing");
    const session: EditSession = { target, span, input, closed: false };
    this.#session = session;
    input.addEventListener("input", () => this.#validate(session));
    picker.addEventListener("input", () => {
      input.value = picker.value;
      this.#validate(session);
    });
    picker.addEventListener("change", () => this.#choose(session, picker.value));
    this.#validate(session);
    this.#placeCard(span, this.#popover);
    input.focus({ preventScroll: true });
    if (caret === undefined) {
      input.select();
    } else {
      input.setSelectionRange(caret.start, caret.end);
    }
  }

  #onPopoverKey(event: KeyboardEvent): void {
    // The popover's keys are its own, not the reading's or the host's.
    event.stopPropagation();
    const session = this.#session;
    if (session === undefined || session.field !== undefined) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      this.#cancel(session, true);
      return;
    }
    if (session.options !== undefined) {
      const shown = session.options.filter((option) => !option.hidden);
      const at = session.active === undefined ? -1 : shown.indexOf(session.active);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const next = shown[Math.max(0, Math.min(shown.length - 1, at + (event.key === "ArrowDown" ? 1 : -1)))];
        this.#setActive(session, next);
      } else if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        if (session.active !== undefined) {
          this.#choose(session, session.active.dataset["name"]!);
        }
        if (event.key === "Tab") {
          this.#openNext(session.target.key, event.shiftKey ? -1 : 1);
        }
      }
      return;
    }
    if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      const value = (session.input?.value ?? "").trim();
      if (!isHexColor(value)) {
        this.#validate(session);
        return;
      }
      this.#choose(session, value);
      if (event.key === "Tab") {
        this.#openNext(session.target.key, event.shiftKey ? -1 : 1);
      }
    }
  }

  /** Closes a popover with `value` chosen. */
  #choose(session: EditSession, value: string): void {
    this.#endSession(session, true);
    this.#commitValue(session.target, value);
  }

  // ---- Dragging a number's name -----------------------------------------------------------

  #onPointerDown(event: PointerEvent): void {
    if (!this.#editable || event.button !== 0) {
      return;
    }
    const name = (event.target as Element).closest?.(".scrub") as HTMLElement | null | undefined;
    const target = name === null || name === undefined ? undefined : this.#targets.get(name.dataset["scrubKey"] ?? "");
    if (target === undefined || target.kind !== "number" || stepNumber(this.#currentOf(target), 0) === undefined) {
      return;
    }
    this.#scrub = { target, name: name!, pointer: event.pointerId, x: event.clientX, from: this.#currentOf(target), value: undefined };
  }

  #onPointerMove(event: PointerEvent): void {
    const scrub = this.#scrub;
    if (scrub === undefined || event.pointerId !== scrub.pointer) {
      return;
    }
    const moved = event.clientX - scrub.x;
    if (scrub.value === undefined) {
      if (Math.abs(moved) < SCRUB_SLOP) {
        return;
      }
      // The press is a drag now: the pointer is the name's until it lets go.
      try {
        scrub.name.setPointerCapture?.(scrub.pointer);
      } catch {
        // A pointer the DOM no longer knows ends with its next event.
      }
      this.#closeCard();
      this.#body.classList.add("scrubbing");
    }
    event.preventDefault();
    const steps = Math.trunc(moved / SCRUB_STEP) * (event.shiftKey ? 10 : 1);
    const value = stepNumber(scrub.from, steps, scrub.target.signed !== false) ?? scrub.from;
    scrub.value = value;
    const span = this.#rendered.get(scrub.target.key);
    if (span !== undefined) {
      showValue(span, scrub.target, this.#writtenOf(scrub.target), value);
    }
  }

  /**
   * Carries a drag on through a reading drawn again, on the same number's new name, showing the
   * number it has reached; when the number is gone, or no longer a number, the drag ends.
   */
  #resumeScrub(scrub: Scrub): void {
    const target = this.#targets.get(scrub.target.key);
    const name = [...this.#body.querySelectorAll<HTMLElement>(".scrub")].find((candidate) => candidate.dataset["scrubKey"] === scrub.target.key);
    if (target === undefined || target.kind !== "number" || name === undefined) {
      return;
    }
    scrub.target = target;
    scrub.name = name;
    this.#scrub = scrub;
    if (scrub.value === undefined) {
      return;
    }
    this.#body.classList.add("scrubbing");
    try {
      name.setPointerCapture?.(scrub.pointer);
    } catch {
      // A pointer the DOM no longer knows ends with its next event.
    }
    const span = this.#rendered.get(target.key);
    if (span !== undefined) {
      showValue(span, target, this.#writtenOf(target), scrub.value);
    }
  }

  /** Ends a drag: its number is committed once, as it was let go, unless the drag was cancelled. */
  #onPointerUp(event: PointerEvent, commit: boolean): void {
    const scrub = this.#scrub;
    if (scrub === undefined || event.pointerId !== scrub.pointer) {
      return;
    }
    this.#scrub = undefined;
    if (scrub.value === undefined) {
      return;
    }
    this.#body.classList.remove("scrubbing");
    this.#ignoreClick = true;
    setTimeout(() => (this.#ignoreClick = false), 0);
    if (commit && scrub.value !== scrub.from) {
      this.#commitValue(scrub.target, scrub.value);
      return;
    }
    // Back where it started: shown as before.
    const span = this.#rendered.get(scrub.target.key);
    if (span !== undefined) {
      if (this.#pending.has(scrub.target.key)) {
        showValue(span, scrub.target, this.#writtenOf(scrub.target), scrub.from);
      } else {
        this.#render();
      }
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
    if (target.closest?.(".row-action") !== null && target.closest?.(".row-action") !== undefined) {
      this.#removeSelected();
      return;
    }
    // The press that ended a drag of a number's name, or a click inside an open field, is not a
    // choice of node.
    if (this.#ignoreClick) {
      this.#ignoreClick = false;
      return;
    }
    if (this.#session?.field?.contains(target) === true) {
      return;
    }
    // A drag that selected text is a text selection, not a choice of node.
    // Chromium reports a selection inside a shadow root only through the root itself.
    const root = this.shadowRoot as (ShadowRoot & { getSelection?(): Selection | null }) | null;
    const textSelection = root?.getSelection?.() ?? this.ownerDocument.getSelection?.();
    if (textSelection !== null && textSelection !== undefined && !textSelection.isCollapsed) {
      return;
    }
    // A click on a value of the unit already selected opens its editor, or toggles a boolean:
    // select, then change.
    const value = this.#editable ? (target.closest?.(".editable-value") as HTMLElement | null | undefined) : undefined;
    if (value !== null && value !== undefined && this.#body.contains(value) && keyOf(this.#unitOf(value) ?? value) === this.#selection) {
      this.#openEditor(keyOf(value)!);
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
      return;
    }
    if (event.key === "Escape") {
      this.#closeCard();
      return;
    }
    // Enter or F2 opens the selected unit's first value, Space toggles its boolean, and Delete or
    // Backspace leaves out its property. Ctrl or Cmd with Z or Y are the host's, for its undo.
    if (this.#editable && !event.ctrlKey && !event.metaKey && !event.altKey) {
      if (event.key === "Enter" || event.key === "F2") {
        if (this.#openSelected(event.key === "Enter")) {
          event.preventDefault();
        }
        return;
      }
      if (event.key === " ") {
        const unit = this.#selectedUnit();
        const toggle = unit === undefined ? undefined : this.#unitValues(unit).find((value) => this.#targets.get(keyOf(value)!)?.kind === "boolean");
        if (toggle !== undefined) {
          event.preventDefault();
          this.#openEditor(keyOf(toggle)!);
        }
        return;
      }
      if (event.key === "Delete" || event.key === "Backspace") {
        if (this.#removeSelected()) {
          event.preventDefault();
        }
        return;
      }
    }
    // `i` shows the selected unit's card, and tells a screen reader what it says.
    if (event.key === "i" && !event.ctrlKey && !event.metaKey && !event.altKey && this.#selection !== undefined) {
      const drawn = this.#rendered.get(this.#selection);
      const anchor = drawn?.matches(".sheet") === true ? (drawn.querySelector<HTMLElement>(":scope > .sheet-head") ?? drawn) : drawn;
      if (anchor !== undefined) {
        event.preventDefault();
        const said = this.#openCard(this.#selection, anchor, true);
        if (said === undefined) {
          // A card for anything else would seem to answer for this.
          this.#closeCard();
        }
        this.#announcer.textContent = said ?? "No information";
      }
    }
  }

  /** The name or value `target` is in, which the reading marks with the key of what it describes. */
  #hoverTargetOf(target: EventTarget | null): HTMLElement | undefined {
    const found = (target as Element | null)?.closest?.("[data-hover]") as HTMLElement | null | undefined;
    return found !== null && found !== undefined && this.#body.contains(found) ? found : undefined;
  }

  /**
   * Notes that the pointer is over `target`, or over nothing that has a card: the card opens once it
   * has rested there, and an open card for anything else closes after a moment's grace.
   */
  #point(target: HTMLElement | undefined): void {
    const key = target?.dataset["hover"];
    if (key === this.#pointedKey) {
      return;
    }
    this.#pointedKey = key;
    clearTimeout(this.#cardOpenTimer);
    if (this.#session !== undefined || this.#scrub?.value !== undefined) {
      return;
    }
    if (key !== undefined && key === this.#cardKey) {
      clearTimeout(this.#cardCloseTimer);
      return;
    }
    if (!this.#card.hidden && !this.#cardByKeyboard) {
      this.#closeCardSoon();
    }
    if (target !== undefined && key !== undefined) {
      this.#cardOpenTimer = setTimeout(() => this.#openCard(key, target, false), CARD_DELAY);
    }
  }

  /**
   * Opens the card of the node with `key` against `anchor`, which draws it, and answers what the card
   * says, as words. Nothing opens, and nothing is answered, when the tree says nothing about it.
   */
  #openCard(key: string, anchor: HTMLElement, byKeyboard: boolean): string | undefined {
    const index = this.#index;
    const at = index?.byKey.get(key);
    const card = index === undefined || at === undefined ? undefined : cardOf(index, at);
    if (card === undefined || !this.#body.contains(anchor)) {
      return undefined;
    }
    clearTimeout(this.#cardCloseTimer);
    const document = this.ownerDocument;
    const signature = document.createElement("div");
    signature.className = "card-signature";
    signature.textContent = card.signature;
    this.#card.replaceChildren(signature);
    if (card.doc !== undefined) {
      const doc = document.createElement("div");
      doc.className = "card-doc";
      doc.append(this.#renderDoc(card.doc));
      this.#card.append(doc);
    }
    this.#cardKey = key;
    this.#cardByKeyboard = byKeyboard;
    this.#card.hidden = false;
    this.#placeCard(anchor);
    // A scroller inside a host's shadow root does not tell the document it scrolled.
    const scroller = this.#scrollPort().element;
    if (scroller !== this.#cardScroller) {
      this.#cardScroller?.removeEventListener("scroll", this.#onDocumentScroll);
      this.#cardScroller = scroller;
      scroller.addEventListener("scroll", this.#onDocumentScroll);
    }
    const doc = (this.#card.querySelector(".card-doc")?.textContent ?? "").trim();
    return `${card.signature}${doc === "" ? "" : `. ${doc}`}`.replace(/\s+/g, " ");
  }

  /** A doc's Markdown as the host renders it, or as the built-in renderer does. */
  #renderDoc(markdown: string): Node {
    if (this.#markdown !== undefined) {
      try {
        const rendered = this.#markdown(markdown) as Node | null | undefined;
        if (rendered !== null && rendered !== undefined && typeof rendered.nodeType === "number") {
          return rendered;
        }
      } catch {
        // A host renderer that fails leaves the doc to the built-in one.
      }
    }
    return renderMarkdown(this.ownerDocument, markdown);
  }

  /**
   * Places the card, or a popover, below `anchor`, or above it where there is no room below, inside
   * the element's box and what of it is in view.
   */
  #placeCard(anchor: HTMLElement, card: HTMLElement = this.#card): void {
    // Measured from the frame's corner, so its width is not cut by where it last stood.
    card.style.left = "0px";
    card.style.top = "0px";
    const frame = this.#frame.getBoundingClientRect();
    const box = this.getBoundingClientRect();
    const port = this.#scrollPort();
    const top = Math.max(box.top, port.top);
    // A popover may run past the reading's end, over the room below it, as a menu does.
    const bottom = card === this.#card ? Math.min(box.bottom, port.top + port.height) : port.top + port.height;
    const target = anchor.getBoundingClientRect();
    const size = card.getBoundingClientRect();
    let y = target.bottom + CARD_GAP;
    if (y + size.height > bottom && target.top - CARD_GAP - size.height >= top) {
      y = target.top - CARD_GAP - size.height;
    }
    y = Math.max(top, Math.min(y, bottom - size.height));
    const x = Math.max(box.left, Math.min(target.left, box.right - size.width));
    card.style.left = `${x - frame.left}px`;
    card.style.top = `${y - frame.top}px`;
  }

  /**
   * Hides the card after a moment, unless the pointer reaches it or what it describes first. A card
   * waiting to open for what the pointer moved onto still opens.
   */
  #closeCardSoon(): void {
    clearTimeout(this.#cardCloseTimer);
    this.#cardCloseTimer = setTimeout(() => this.#hideCard(), CARD_GRACE);
  }

  /** Closes the card, and any card waiting to open: on Escape, a scroll, a click or a new reading. */
  #closeCard(): void {
    clearTimeout(this.#cardOpenTimer);
    this.#hideCard();
  }

  #hideCard(): void {
    clearTimeout(this.#cardCloseTimer);
    this.#cardScroller?.removeEventListener("scroll", this.#onDocumentScroll);
    this.#cardScroller = undefined;
    this.#cardKey = undefined;
    this.#cardByKeyboard = false;
    if (this.#card !== undefined && !this.#card.hidden) {
      this.#card.hidden = true;
      this.#card.replaceChildren();
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
