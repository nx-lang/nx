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

import { cardOf, propertyCardOf, type Card } from "./card.js";
import {
  editOf,
  editTargets,
  isHexColor,
  isNumericLiteral,
  isRemovable,
  isWholeValue,
  removalOf,
  runProblem,
  unwrittenProperties,
  type EditTarget,
  type UnwrittenProperty
} from "./edit.js";
import {
  caseOptions,
  choiceMatches,
  choiceRows,
  choicesOf,
  defaultShown,
  defaultValue,
  offsetAtPoint,
  pickerColor,
  rangeProblem,
  showValue,
  stepNumber,
  valueHost
} from "./editors.js";
import { renderMarkdown } from "./markdown.js";
import { defaultFolded, readingRuns, renderSource, type NxViewerColor, type NxViewerIcons, type ReadingRun } from "./source.js";
import { styles } from "./styles.js";
import { elementProperties, indexTree, nodeOf, type TreeIndex } from "./tree.js";

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

/**
 * The event fired when the open editor comes to hold an entry that differs from its value, and when
 * it no longer does, as when it commits, cancels or closes, so a host can offer Undo for an edit not
 * yet committed.
 */
export const NX_DRAFT_EVENT = "nx-draft";

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

/** What the `nx-draft` event carries. */
export interface NxDraftDetail {
  /** Whether the open editor holds an entry that differs from its value. */
  readonly changed: boolean;
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

/** How many unwritten properties a selected element lists before a filter field leads them. */
const MORE_FILTER_PAST = 12;

/** How far the pointer moves sideways, in pixels, for a number's name dragged to step it once. */
const SCRUB_STEP = 2;

/** How far the pointer moves before a press on a number's name becomes a drag, in pixels. */
const SCRUB_SLOP = 3;

/**
 * How far the pointer moves before a press on a stepper becomes a drag, in pixels: more than a name's,
 * since a stepper is clicked far more often than dragged, and a hand rarely keeps still on so small a
 * target.
 */
const STEPPER_SLOP = 6;

/** How far the pointer moves up or down, in pixels, for a stepper dragged to step its number once. */
const STEPPER_STEP = 4;

/** How long the pointer rests on a name or a value before its card opens, in milliseconds. */
const CARD_DELAY = 500;

/** How long a card stays once the pointer leaves it and what it describes, to reach it. */
const CARD_GRACE = 300;

/** The room between a card and what it describes, in pixels. */
const CARD_GAP = 4;
/** How long typing pauses before an invalid entry says why, in milliseconds. */
const PROBLEM_DELAY = 400;

/**
 * How long, in milliseconds, a reading left with no tree keeps the height it had, for a host that
 * gives the next tree a little later.
 */
const EMPTY_HOLD = 1000;
/** How far beside its editor an invalid entry says why, in pixels. */
const PROBLEM_GAP = 8;

// A module imported where there is no DOM still loads; the element is only defined where
// `HTMLElement` exists.
const ElementBase = (typeof HTMLElement === "undefined" ? class {} : HTMLElement) as typeof HTMLElement;

/**
 * What a click selects, and what the pointer and the arrow keys move between: an element on lines
 * of its own (through its header), a row of a group, an element on one line, an element written as
 * its tags that starts a line, or a block written as it is, as an `if` or a `for`. A group's header
 * stands for its group.
 */
const UNITS = ".sheet-head, .sheet-row:not(.more), .line-element, .tag-line, .block-line";

/** The units as they are drawn selected: a group stands for itself, not through its header. */
const DRAWN_UNITS = ".sheet, .sheet-row:not(.more), .line-element, .tag-line, .block-line";

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
    if (child.classList.contains("row-menu")) {
      // The row's menu mark is a button of its own, not part of what the row says.
      return;
    }
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
  /** The rows of the choices listed with a field or a color, their list, and the one Enter picks. */
  choices?: HTMLElement[];
  choiceList?: HTMLElement;
  choiceActive?: HTMLElement | undefined;
  /** Whether a field's choices are listed under it; a color's always are. */
  choicesOpen?: boolean;
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

/** A stepper pressed on a number, and dragged up or down once the press became a drag. */
interface Stepping {
  target: EditTarget;
  readonly pointer: number;
  /** Where the press began, and, once it became a drag, where the drag began. */
  y: number;
  /** Which stepper: up, `1`, or down, `-1`. */
  readonly direction: 1 | -1;
  /** The number when the press began. */
  readonly from: string;
  /** The number now, once the press became a drag. */
  value: string | undefined;
}

/** A stepper's tooltip, which names the keys that do the same. */
const STEPPER_TITLES = {
  up: "Step up (\u2191). Shift+click steps 10. Drag up or down to scrub.",
  down: "Step down (\u2193). Shift+click steps 10. Drag up or down to scrub."
} as const;

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
  /** Lets go of the height an empty reading holds, once no tree has come for a while. */
  #holdTimer: ReturnType<typeof setTimeout> | undefined;
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
  /** The value whose popover a press on it closed, which the click that follows leaves closed. */
  #closedOnPress: string | undefined;
  /** A stepper pressed, and dragged up or down once the press became a drag. */
  #stepping: Stepping | undefined;
  /** The key of the element whose unwritten properties are listed. */
  #moreOpen: string | undefined;
  /** What the filter of the unwritten properties holds. */
  #moreFilter = "";
  /** What the list of unwritten properties was drawn for: the element's key and whether it was open. */
  #moreFor: string | undefined;
  /** The rows of unwritten properties drawn, and the block that holds them under a line. */
  #moreDrawn: HTMLElement[] = [];
  #moreBlock: HTMLElement | undefined;
  /** The keys of the unwritten properties' values, which the reading edits as written ones. */
  #moreKeys: string[] = [];
  /** The cards of the unwritten properties' names, by the key their attribute would have. */
  #moreCards = new Map<string, Card>();
  /** The menu of a row's actions, when open: the row's key. */
  #menuKey: string | undefined;

  readonly #badge: HTMLSpanElement;
  readonly #body: HTMLDivElement;
  /** The highlights of the selected and the hovered unit, drawn behind the text. */
  readonly #highlights: HTMLDivElement;
  /** Says what the arrow keys selected, for a screen reader, since focus stays on the body. */
  readonly #announcer: HTMLDivElement;
  readonly #frame: HTMLDivElement;
  /** The hover card: what a name or a value under the pointer is, and its doc. */
  readonly #card: HTMLDivElement;
  /** The popover a case or a color is chosen in, or a field's choices are listed in. */
  readonly #popover: HTMLDivElement;
  /** A number's steppers and a field's choices chevron, drawn past the value they act on. */
  readonly #tools: HTMLSpanElement;
  /** The value the steppers were drawn past while the pointer rests on it, with no editor open. */
  #toolsHover: string | undefined;
  /** The menu of a row's actions. */
  readonly #menu: HTMLDivElement;
  /** Whether the open editor holds an entry that differs from its value, as `nx-draft` last said. */
  #draftChanged = false;
  /** Why the open editor cannot take its entry, under it while it cannot. */
  readonly #problem: HTMLDivElement;
  #problemTimer: ReturnType<typeof setTimeout> | undefined;
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
    this.#closedOnPress = undefined;
    // A press on the open menu's own mark leaves it to the click that follows, which closes it.
    const onMark = event
      .composedPath()
      .some(
        (node) =>
          (node as Element).classList?.contains("row-menu") === true &&
          (node as Element).closest<HTMLElement>(".sheet-row[data-key]")?.dataset["key"] === this.#menuKey
      );
    if (this.#menuKey !== undefined && !onMark && !event.composedPath().includes(this.#menu)) {
      this.#closeMenu(false);
    }
    if (session !== undefined && session.field === undefined && !event.composedPath().includes(this.#popover)) {
      this.#cancel(session, false);
      // A press on the value whose popover was open closes it, as a dropdown's does; the click
      // that follows does not open it again.
      if (event.composedPath().includes(session.span)) {
        this.#closedOnPress = session.target.key;
      }
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
      if (option === null || session === undefined) {
        return;
      }
      if (option.classList.contains("choice")) {
        this.#pickChoice(session, option);
      } else if (session.options !== undefined) {
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

    // The tools are drawn in the reading, just past the value they act on, over the gap after it,
    // so nothing moves; they keep the pointer's presses to themselves.
    this.#tools = document.createElement("span");
    this.#tools.className = "value-tools";
    this.#tools.setAttribute("aria-hidden", "true");
    const steppers = document.createElement("span");
    steppers.className = "steppers";
    for (const direction of ["up", "down"] as const) {
      const stepper = document.createElement("span");
      stepper.className = `stepper ${direction}`;
      stepper.title = STEPPER_TITLES[direction];
      steppers.append(stepper);
    }
    const toggle = document.createElement("span");
    toggle.className = "choices-toggle";
    toggle.title = "Choices (Alt+\u2193)";
    this.#tools.append(steppers, toggle);
    this.#tools.addEventListener("pointerdown", (event) => this.#onToolsPointerDown(event));
    this.#tools.addEventListener("pointermove", (event) => this.#onToolsPointerMove(event));
    this.#tools.addEventListener("pointerup", (event) => this.#onToolsPointerUp(event, true));
    this.#tools.addEventListener("pointercancel", (event) => this.#onToolsPointerUp(event, false));
    for (const name of ["click", "dblclick", "mousedown"]) {
      this.#tools.addEventListener(name, (event) => {
        event.stopPropagation();
        event.preventDefault();
      });
    }

    this.#problem = document.createElement("div");
    this.#problem.className = "edit-problem";
    this.#problem.id = "edit-problem";
    this.#problem.hidden = true;

    this.#menu = document.createElement("div");
    this.#menu.className = "menu";
    this.#menu.setAttribute("role", "menu");
    this.#menu.hidden = true;
    this.#menu.addEventListener("keydown", (event) => this.#onMenuKey(event));
    this.#menu.addEventListener("pointerdown", (event) => event.preventDefault());
    this.#menu.addEventListener("click", (event) => {
      const item = (event.target as Element).closest?.(".menu-item") as HTMLElement | null;
      if (item !== null) {
        this.#chooseMenuItem(item);
      }
    });
    frame.append(toolbar, this.#body, this.#announcer, this.#card, this.#popover, this.#problem, this.#menu);
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
      this.#hoverTools(event.target as Element);
    });
    this.#body.addEventListener("pointerleave", () => {
      this.#setHoveredUnit(undefined);
      this.#point(undefined);
      this.#hoverTools(null);
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
      // An unwritten property's value is not in the tree.
      if (element.getClientRects().length === 0 || element.closest(".more") !== null) {
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
    // A press on a stepper goes on with the next tree, on the same number.
    const stepping = this.#stepping;
    this.#dropSession();
    this.#pointedKey = undefined;
    this.#rendered.clear();
    this.#hoveredUnit = undefined;
    // The reading keeps the height it had while it is drawn again, so the shorter drafts the fitting
    // below measures, or no tree between two, do not pull up what scrolls it: after an edit near
    // the bottom, the reader stays where they were.
    clearTimeout(this.#holdTimer);
    if (this.#body.style.minHeight === "" && this.#body.childElementCount > 0) {
      this.#body.style.minHeight = `${this.#body.offsetHeight}px`;
    }
    this.#body.replaceChildren();
    this.#body.classList.toggle("editable", this.#editable);
    this.#index = undefined;
    this.#targets = new Map();
    this.#moreDrawn = [];
    this.#moreBlock = undefined;
    this.#moreKeys = [];
    this.#moreFor = undefined;
    this.#closeMenu(false);
    if (tree === undefined) {
      // A host that gives the next tree soon, at once or once it has it, keeps the height; one that
      // leaves the reading empty lets it go.
      this.#holdTimer = setTimeout(() => {
        if (this.#tree === undefined) {
          this.#body.style.minHeight = "";
        }
      }, EMPTY_HOLD);
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
    const targets = editTargets(index);
    const draw = (wrapped?: ReadonlySet<string>): void => {
      this.#body.replaceChildren(
        renderSource(this.ownerDocument, index, this.#colors, { wrapped, icons: this.#icons, folded: this.#folded, turning: this.#turning })
      );
    };
    try {
      draw();
      // Each element whose properties do not fit on one line gives each a row, drawn again until
      // all left on lines fit. Hidden or unattached, nothing is measured, and the size the reading
      // takes when shown draws it again.
      this.#fitWidth = this.#body.clientWidth;
      const wrapped = new Set<string>();
      // A property's element value on a line or a header that does not fit is measured once that
      // one has its rows, so each pass settles one level of values, and values nested deeper than
      // this keep their innermost lines.
      for (let pass = 0; pass < 6 && this.#fitWidth > 0; pass++) {
        const found = this.#wrapped();
        if (found.length === 0) {
          break;
        }
        found.forEach((key) => wrapped.add(key));
        draw(wrapped);
      }
    } finally {
      this.#body.style.minHeight = "";
    }
    for (const element of this.#body.querySelectorAll<HTMLElement>("[data-key]")) {
      this.#rendered.set(element.dataset["key"]!, element);
    }
    if (this.#editable) {
      this.#targets = targets;
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
    this.#body.append(this.#highlights);
    this.#paintHighlights();
    this.#turnChevron();
    const reopen = this.#reopen;
    this.#reopen = undefined;
    if (reopen !== undefined && this.#targets.has(reopen.key) && this.#rendered.has(reopen.key)) {
      // Where the caret was cannot always be read; then it goes after what the reader typed, which a
      // reopened editor never selects, so the next key does not replace it.
      const end = reopen.draft?.length ?? 0;
      this.#openEditor(reopen.key, reopen.draft, reopen.caret ?? { start: end, end });
    }
    if (stepping !== undefined) {
      this.#resumeStepping(stepping);
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
    // The rows of an element's unwritten properties are the element's, wherever they are drawn.
    const more = target?.closest?.(".more") as HTMLElement | null | undefined;
    if (more !== null && more !== undefined && this.#body.contains(more)) {
      return this.#rendered.get(more.dataset["moreOf"] ?? "");
    }
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
    const draw = (from: HTMLElement, rows: DOMRect, className: string, reach = 0): void => {
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
      // Rows drawn under a line element reach as far as the widest of them.
      const right =
        tight === undefined ? frame.width - 8 : Math.max(tight.right, reach) - frame.left + pad + this.#body.scrollLeft;
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
      // A line's unwritten properties, listed under it, are part of what is selected.
      if (this.#moreBlock !== undefined && this.#body.contains(this.#moreBlock)) {
        // The rows' cells stretch across the block, so how far they reach is read from what they hold.
        const reach = [...this.#moreBlock.querySelectorAll<HTMLElement>(".more-label, .more-filter, .sheet-name, .more-value, .more-default")]
          .filter((part) => part.closest<HTMLElement>(".sheet-row")?.hidden !== true)
          .map((part) => {
            // A cell may stretch across its grid; its text's own box says where it ends.
            if (part.childNodes.length === 0) {
              return part.getBoundingClientRect().right;
            }
            const range = this.ownerDocument.createRange();
            range.selectNodeContents(part);
            return range.getBoundingClientRect().right;
          });
        draw(unit, this.#moreBlock.getBoundingClientRect(), "selected-body", Math.max(0, ...reach));
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
    // Selecting another unit closes the list of an element's unwritten properties.
    if (key !== this.#moreOpen) {
      this.#moreOpen = undefined;
      this.#moreFilter = "";
    }
    if (this.#menuKey !== undefined && this.#menuKey !== key) {
      this.#closeMenu(false);
    }
    // Steppers drawn while the pointer rested on a value belong to the row that was selected.
    if (this.#session === undefined && this.#stepping === undefined && this.#tools.isConnected) {
      this.#hideTools();
    }
    this.#selectNode(key, fire, reveal);
    this.#drawMore();
  }

  #selectNode(key: string | undefined, fire: boolean, reveal: boolean): void {
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
    // Each written row has a menu of what can be done with it, opened from a quiet mark at its end.
    for (const row of this.#body.querySelectorAll<HTMLElement>(".sheet-row[data-key]")) {
      const mark = this.ownerDocument.createElement("span");
      mark.className = "row-menu";
      mark.textContent = "\u22EF";
      mark.setAttribute("role", "button");
      mark.setAttribute("aria-haspopup", "menu");
      mark.setAttribute("aria-label", `Actions for ${index.tree.nodes[index.byKey.get(row.dataset["key"]!)!]?.name ?? "the row"}`);
      row.append(mark);
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
      // An unwritten property's value is the list's, not the unit's own.
      if (this.#unitOf(value) === unit && value.closest(".more") === null && !found.some((other) => keyOf(other) === keyOf(value))) {
        found.push(value);
      }
    }
    return found;
  }

  /** The text of the value `target` names as the tree writes it; nothing for a property not written. */
  #writtenOf(target: EditTarget): string {
    return target.adding === true ? "" : (this.#index?.tree.nodes[target.at]?.value ?? "");
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
    this.#closeMenu(false);
    const current = this.#currentOf(target);
    switch (target.kind) {
      case "boolean":
        this.#commitValue(target, current === "true" ? "false" : "true");
        return;
      case "case":
        this.#openCases(target, span, draft ?? "", caret);
        break;
      case "color":
        this.#openColor(target, span, draft ?? current, caret);
        break;
      default:
        this.#openField(target, span, draft ?? current, caret);
    }
    const session = this.#session;
    if (session !== undefined && session.target === target) {
      this.#listChoices(session);
      this.#showTools(session.span, target);
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
    if (field === undefined || selection === null || selection === undefined) {
      return undefined;
    }
    // Where the browser keeps no selection apart for a shadow root, its composed ranges reach inside.
    // An engine that takes them in another form throws, and the selection's own range is read.
    let composed: StaticRange | undefined;
    try {
      composed = (
        selection as Selection & { getComposedRanges?(options: { shadowRoots: ShadowRoot[] }): StaticRange[] }
      ).getComposedRanges?.({ shadowRoots: this.shadowRoot === null ? [] : [this.shadowRoot] })[0];
    } catch {
      composed = undefined;
    }
    const range = composed ?? (selection.rangeCount === 0 ? undefined : selection.getRangeAt(0));
    if (range === undefined || !field.contains(range.startContainer) || !field.contains(range.endContainer)) {
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
    // A placeholder reads, quieter, while the field is empty, and is never written.
    // An unset property's field keeps reading the type its place showed until something is typed.
    const placeholder =
      (target.kind === "text" ? target.hints?.placeholder : undefined) ?? (target.adding === true ? span.dataset["placeholder"] : undefined);
    if (placeholder !== undefined && placeholder !== "") {
      field.dataset["placeholder"] = placeholder;
      field.setAttribute("aria-placeholder", placeholder);
    }
    if (target.kind === "text" && target.hints?.multiline === true) {
      field.classList.add("field-multiline");
      field.setAttribute("aria-multiline", "true");
    }
    host.replaceChildren(...keep, ...(lead === undefined ? [] : [lead]), field, ...(trail === undefined ? [] : [trail]));
    span.classList.add("editing");
    const session: EditSession = { target, span, host, restore, field, closed: false };
    this.#session = session;
    field.addEventListener("keydown", (event) => this.#onFieldKey(event, session));
    field.addEventListener("input", () => {
      this.#validate(session);
      // Typing lists the choices that hold what is typed.
      this.#filterChoices(session, field.textContent ?? "", true);
    });
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
    if (target.property !== undefined) {
      return target.property.name;
    }
    const index = this.#index!;
    const owner = index.tree.nodes[target.at]!.parent;
    const name = owner === undefined ? undefined : index.tree.nodes[owner]!.name;
    return name ?? (target.kind === "run" ? "Text" : "Value");
  }

  /** Why the field's entry cannot be taken, or nothing when it can. */
  #problemOf(target: EditTarget, value: string): string | undefined {
    switch (target.kind) {
      case "number": {
        const entry = value.trim();
        if (!isNumericLiteral(entry)) {
          return "Not a number";
        }
        if (target.signed === false && entry.startsWith("-")) {
          return "Cannot be negative here";
        }
        return rangeProblem(entry, target.hints);
      }
      case "run":
        return runProblem(value);
      case "color":
        return isHexColor(value.trim()) ? undefined : "Not a hex color";
      default:
        return undefined;
    }
  }

  /**
   * Marks the editor's entry as invalid while the editor cannot take it, and says why under it: once
   * typing pauses, so a number half typed, as `-`, does not flash a message, or at once, `now`, as
   * when Enter is pressed on it.
   */
  #validate(session: EditSession, now = false): void {
    const input = session.field ?? session.input;
    if (input === undefined || session.target.kind === "case") {
      return;
    }
    this.#reportDraft(this.#draftOf(session) !== this.#currentOf(session.target));
    const problem = this.#problemOf(session.target, this.#draftOf(session) ?? "");
    input.classList.toggle("invalid", problem !== undefined);
    input.setAttribute("aria-invalid", String(problem !== undefined));
    clearTimeout(this.#problemTimer);
    if (problem === undefined) {
      this.#hideProblem();
      input.removeAttribute("aria-describedby");
      return;
    }
    input.setAttribute("aria-describedby", this.#problem.id);
    if (now || !this.#problem.hidden) {
      this.#showProblem(session, problem);
    } else {
      this.#problemTimer = setTimeout(() => this.#showProblem(session, problem), PROBLEM_DELAY);
    }
  }

  /** Shows why `session`'s entry is invalid, under its field, or under its popover. */
  #showProblem(session: EditSession, problem: string): void {
    if (session.closed) {
      return;
    }
    this.#problem.textContent = problem;
    this.#problem.hidden = false;
    // Beside its field or popover where the reading has room, so it covers nothing it would explain;
    // otherwise under it, as a card.
    const anchor = session.field ?? this.#popover;
    const frame = this.#frame.getBoundingClientRect();
    const target = anchor.getBoundingClientRect();
    const size = this.#problem.getBoundingClientRect();
    const x = target.right + PROBLEM_GAP;
    if (x + size.width <= this.getBoundingClientRect().right) {
      this.#problem.style.left = `${x - frame.left}px`;
      this.#problem.style.top = `${target.top + (target.height - size.height) / 2 - frame.top}px`;
    } else {
      this.#placeCard(session.field === undefined ? this.#popover : session.span, this.#problem);
    }
  }

  /** Fires `nx-draft` when whether the open editor holds a changed entry changes. */
  #reportDraft(changed: boolean): void {
    if (changed === this.#draftChanged) {
      return;
    }
    this.#draftChanged = changed;
    this.dispatchEvent(new CustomEvent<NxDraftDetail>(NX_DRAFT_EVENT, { detail: { changed }, bubbles: true, composed: true }));
  }

  #hideProblem(): void {
    clearTimeout(this.#problemTimer);
    this.#problem.hidden = true;
    this.#problem.textContent = "";
  }

  #onFieldKey(event: KeyboardEvent, session: EditSession): void {
    // A field's keys are its own, Ctrl+Z and Ctrl+Y included, not the reading's or the host's.
    event.stopPropagation();
    const field = session.field!;
    const target = session.target;
    // The choices listed with the field: Alt+Down lists them and Alt+Up hides them; listed, the
    // arrows walk them, and otherwise a number's arrows step it.
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && (session.choices?.length ?? 0) > 0) {
      if (event.altKey) {
        event.preventDefault();
        this.#toggleChoices(session, event.key === "ArrowDown");
        return;
      }
      if (session.choicesOpen === true) {
        event.preventDefault();
        this.#stepChoice(session, event.key === "ArrowDown" ? 1 : -1);
        return;
      }
    }
    switch (event.key) {
      case "Enter":
        event.preventDefault();
        if (session.choicesOpen === true && session.choiceActive !== undefined) {
          this.#pickChoice(session, session.choiceActive);
          return;
        }
        // Shift+Enter breaks a string's line; in a field hinted for several lines, Enter does, and
        // Ctrl or Cmd+Enter commits.
        if (target.kind === "text" && (event.shiftKey || (target.hints?.multiline === true && !event.ctrlKey && !event.metaKey))) {
          this.#insertLineBreak(session);
          return;
        }
        this.#finishField(session, "key");
        return;
      case "Escape":
        event.preventDefault();
        // Escape hides the choices first, then cancels.
        if (session.choicesOpen === true) {
          this.#toggleChoices(session, false);
          return;
        }
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
        const stepped = stepNumber((field.textContent ?? "").trim(), steps, target.signed !== false, target.hints);
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
    if (value === this.#currentOf(target)) {
      this.#endSession(session, how === "key");
      session.host!.replaceChildren(...session.restore!);
      return true;
    }
    if (this.#problemOf(target, value) !== undefined) {
      if (how === "blur") {
        this.#cancel(session, false);
        return true;
      }
      this.#validate(session, true);
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
    this.#hideProblem();
    this.#reportDraft(false);
    // A field's popover lists its choices; a popover editor is the popover itself.
    this.#closePopover();
    this.#hideTools();
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
    this.#hideProblem();
    this.#reportDraft(false);
    this.#closePopover();
    this.#hideTools();
    this.#stepping = undefined;
    this.#body.classList.remove("stepping");
  }

  /** Hides the popover and drops its contents and its kind, so nothing of it stays on screen. */
  #closePopover(): void {
    this.#popover.hidden = true;
    this.#popover.className = "popover";
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
      this.#markAdded(target, span);
    }
    this.#fire(NX_EDIT_EVENT, { key: this.#editKeyOf(target), edit: editOf(target, value) });
  }

  /** The key an edit of `target` is aimed at: its node's, or, for a property it adds, its element's. */
  #editKeyOf(target: EditTarget): string {
    return target.adding === true && target.property !== undefined ? target.property.element : target.key;
  }

  /** An unwritten property just set reads as written, where it stands, until the host's tree. */
  #markAdded(target: EditTarget, span: HTMLElement): void {
    if (target.adding !== true) {
      return;
    }
    const row = span.closest(".dimmed-row");
    row?.classList.replace("dimmed-row", "added-row");
    row?.querySelector(".more-default")?.remove();
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
   * value the reading cannot edit, as a row's expression, and with `source` for any row's value, as
   * its menu's Show in Source does. Answers whether either happened.
   */
  #openSelected(open: boolean, source = false): boolean {
    const unit = this.#selectedUnit();
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    const first = source ? undefined : unit !== undefined ? this.#unitValues(unit)[0] : selected?.matches(".editable-value") === true ? selected : undefined;
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
    if (this.#session?.field?.contains(target) === true || target.closest?.(".fold") != null) {
      return;
    }
    const opens = target.closest?.(".dimmed-row[data-opens]") as HTMLElement | null | undefined;
    if (opens !== null && opens !== undefined && this.#body.contains(opens)) {
      this.#openMoreInSource(opens);
      return;
    }
    if (target.closest?.(".more") != null) {
      if (target.closest?.(".editable-value") == null) {
        return;
      }
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
    if (!isRemovable(index, at)) {
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
    // Segmented, a few short cases read as one row of buttons; more than five, or a row too wide
    // for the reading, read as the list.
    const segmented = target.hints?.kind === "segmented" && options.length <= 5;
    this.#popover.className = segmented ? "popover cases segmented" : "popover cases";
    this.#popover.replaceChildren(input, list);
    this.#popover.hidden = false;
    if (segmented) {
      list.setAttribute("aria-orientation", "horizontal");
      const room = this.#body.clientWidth;
      if (room > 0 && list.scrollWidth > room) {
        this.#popover.className = "popover cases";
        list.removeAttribute("aria-orientation");
      }
    }
    span.classList.add("editing");
    const session: EditSession = { target, span, input, options, active: undefined, closed: false };
    this.#session = session;
    input.addEventListener("input", () => this.#filterCases(session));
    this.#filterCases(session, options.find((option) => option.dataset["name"] === current));
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
    // Placed before an option is brought into view, which then scrolls the list alone.
    this.#placeCard(session.span, this.#popover);
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
      // Only the list scrolls: `scrollIntoView` would scroll the reading too, and the page.
      const list = option.parentElement;
      if (list !== null) {
        const shown = option.getBoundingClientRect();
        const port = list.getBoundingClientRect();
        if (shown.top < port.top) {
          list.scrollTop -= port.top - shown.top;
        } else if (shown.bottom > port.bottom) {
          list.scrollTop += shown.bottom - port.bottom;
        }
      }
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
    input.addEventListener("input", () => {
      this.#validate(session);
      this.#filterChoices(session, input.value, true);
    });
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
      // A row of buttons steps sideways too.
      const steps: Record<string, number> = { ArrowDown: 1, ArrowUp: -1, ArrowRight: 1, ArrowLeft: -1 };
      const sideways = event.key === "ArrowRight" || event.key === "ArrowLeft";
      if (steps[event.key] !== undefined && (!sideways || this.#popover.classList.contains("segmented"))) {
        event.preventDefault();
        const next = shown[Math.max(0, Math.min(shown.length - 1, at + steps[event.key]!))];
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
    const target = session.target;
    // A color's choices are listed over its field, and the arrows walk them.
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && (session.choices?.length ?? 0) > 0) {
      event.preventDefault();
      this.#stepChoice(session, event.key === "ArrowDown" ? 1 : -1);
      return;
    }
    if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      if (event.key === "Enter" && session.choiceActive !== undefined) {
        this.#pickChoice(session, session.choiceActive);
        return;
      }
      const raw = session.input?.value ?? "";
      const value = target.kind === "number" ? raw.trim() : raw.trim();
      // A popover left as it was leaves the value as it is.
      if (value === this.#currentOf(target)) {
        this.#cancel(session, true);
      } else if (this.#problemOf(target, value) !== undefined) {
        this.#validate(session, true);
        return;
      } else {
        this.#choose(session, value);
      }
      if (event.key === "Tab") {
        this.#openNext(target.key, event.shiftKey ? -1 : 1);
      }
    }
  }

  /** Closes a popover with `value` chosen. */
  #choose(session: EditSession, value: string): void {
    this.#endSession(session, true);
    this.#commitValue(session.target, value);
  }

  // ---- Choices ------------------------------------------------------------------------------

  /**
   * Lists the choices `session`'s value offers, when its hints give any it takes: under a field, once
   * the reader asks for them or types, and over a color's field at once. The default's row leads
   * them when the property has a default, and the value's own row is marked.
   */
  #listChoices(session: EditSession): void {
    const target = session.target;
    if (target.kind !== "number" && target.kind !== "text" && target.kind !== "color") {
      return;
    }
    const choices = choicesOf(target.hints, target.kind);
    if (choices.length === 0) {
      return;
    }
    const document = this.ownerDocument;
    // A property not written reads as its default until the reader sets it.
    const current = target.adding === true && !this.#pending.has(target.key) ? undefined : this.#currentOf(target);
    const rows = choiceRows(document, target, choices, current, this.#defaultOf(target));
    rows.forEach((row, position) => (row.id = `nx-viewer-choice-${position}`));
    const list = document.createElement("div");
    list.className = choices.some((choice) => choice.label !== undefined) ? "options choices-list labeled" : "options choices-list";
    list.id = "nx-viewer-choices";
    list.setAttribute("role", "listbox");
    list.setAttribute("aria-label", `${this.#labelOf(target)}: choices`);
    list.append(...rows);
    if (rows[0]?.dataset["default"] === "true") {
      const rule = document.createElement("div");
      rule.className = "choices-rule";
      rule.setAttribute("role", "none");
      rule.setAttribute("aria-hidden", "true");
      rows[0].after(rule);
    }
    session.choices = rows;
    session.choiceList = list;
    const input = session.field ?? session.input;
    input?.setAttribute("aria-haspopup", "listbox");
    if (session.field !== undefined) {
      // A field with choices is a combobox; it names the list only while the list is drawn.
      session.field.setAttribute("role", "combobox");
      session.field.setAttribute("aria-autocomplete", "list");
      session.field.setAttribute("aria-expanded", "false");
      return;
    }
    input?.setAttribute("aria-controls", list.id);
    // A color's popover is open already: its choices lead it.
    this.#popover.prepend(list);
    this.#popover.classList.add("with-choices");
    session.choicesOpen = true;
    this.#placeCard(session.span, this.#popover);
  }

  /**
   * The default of `target`'s property as its choices list reads it, the literal its editor holds or
   * the default as written; nothing for a property with no default, or a value of no property.
   */
  #defaultOf(target: EditTarget): string | undefined {
    const index = this.#index;
    const property = target.property;
    const at = property === undefined ? undefined : index?.byKey.get(property.element);
    const element = at === undefined ? undefined : index!.tree.nodes[at];
    const declared = element === undefined ? undefined : elementProperties(index!.tree, element)?.find((candidate) => candidate.name === property!.name);
    const written = declared?.default?.trim();
    if (written === undefined || (target.kind !== "number" && target.kind !== "text" && target.kind !== "color")) {
      return undefined;
    }
    const literal = defaultValue(written, { kind: target.kind });
    return literal !== "" || written === '""' ? literal : written;
  }

  /** Lists `session`'s choices under its field, all of them, or hides them. */
  #toggleChoices(session: EditSession, open: boolean): void {
    if (open) {
      for (const row of session.choices ?? []) {
        row.hidden = false;
      }
      session.choiceList?.querySelector<HTMLElement>(".choices-rule")?.removeAttribute("hidden");
    }
    this.#setChoicesOpen(session, open);
  }

  #setChoicesOpen(session: EditSession, open: boolean): void {
    const list = session.choiceList;
    if (session.field === undefined || list === undefined) {
      return;
    }
    session.choicesOpen = open;
    session.field.setAttribute("aria-expanded", String(open));
    if (open) {
      session.field.setAttribute("aria-controls", list.id);
    } else {
      session.field.removeAttribute("aria-controls");
    }
    this.#tools.classList.toggle("open", open);
    if (!open) {
      this.#setChoiceActive(session, undefined);
      this.#closePopover();
      return;
    }
    if (this.#popover.hidden || !this.#popover.contains(list)) {
      this.#popover.className = "popover choices";
      this.#popover.replaceChildren(list);
      this.#popover.hidden = false;
    }
    this.#placeCard(session.span, this.#popover);
    if (session.choiceActive === undefined) {
      // The value's own row is in view.
      const marked = session.choices?.find((row) => row.classList.contains("current") && !row.hidden);
      if (marked !== undefined) {
        this.#scrollToChoice(marked);
      }
    }
  }

  /**
   * Shows the choices whose label or value holds `typed`; a field's list opens with `open`, as the
   * reader types, and closes when none match.
   */
  #filterChoices(session: EditSession, typed: string, open: boolean): void {
    const rows = session.choices;
    if (rows === undefined) {
      return;
    }
    for (const row of rows) {
      row.hidden = !choiceMatches(row, typed);
    }
    const rule = session.choiceList?.querySelector<HTMLElement>(".choices-rule");
    rule?.toggleAttribute("hidden", rows[0]?.hidden !== false || rows.slice(1).every((row) => row.hidden));
    if (session.choiceActive?.hidden === true) {
      this.#setChoiceActive(session, undefined);
    }
    const none = rows.every((row) => row.hidden);
    if (session.field === undefined) {
      session.choiceList?.toggleAttribute("hidden", none);
      this.#placeCard(session.span, this.#popover);
      return;
    }
    if (none) {
      this.#setChoicesOpen(session, false);
    } else if (open || session.choicesOpen === true) {
      this.#setChoicesOpen(session, true);
    }
  }

  /** Moves the active choice one up or down among those shown; past either end, none is active. */
  #stepChoice(session: EditSession, direction: 1 | -1): void {
    const shown = (session.choices ?? []).filter((row) => !row.hidden);
    if (shown.length === 0) {
      return;
    }
    const at = session.choiceActive === undefined ? -1 : shown.indexOf(session.choiceActive);
    const next = at < 0 ? (direction === 1 ? 0 : shown.length - 1) : at + direction;
    this.#setChoiceActive(session, next < 0 || next >= shown.length ? undefined : shown[next]);
  }

  #setChoiceActive(session: EditSession, row: HTMLElement | undefined): void {
    session.choiceActive = row;
    for (const candidate of session.choices ?? []) {
      candidate.classList.toggle("active", candidate === row);
    }
    const input = session.field ?? session.input;
    if (row === undefined) {
      input?.removeAttribute("aria-activedescendant");
    } else {
      input?.setAttribute("aria-activedescendant", row.id);
      this.#scrollToChoice(row);
    }
  }

  /** Scrolls `row`'s list, and nothing else, so the row is in view. */
  #scrollToChoice(row: HTMLElement): void {
    const list = row.parentElement;
    if (list === null) {
      return;
    }
    const shown = row.getBoundingClientRect();
    const port = list.getBoundingClientRect();
    if (shown.top < port.top) {
      list.scrollTop -= port.top - shown.top;
    } else if (shown.bottom > port.bottom) {
      list.scrollTop += shown.bottom - port.bottom;
    }
  }

  /**
   * Closes `session` with the choice `row` picked: its value written as a literal, or, for the
   * default's row, the property reset as the row menu's Reset does. On a property not written, the
   * default's row adds nothing.
   */
  #pickChoice(session: EditSession, row: HTMLElement): void {
    const target = session.target;
    const value = row.dataset["value"]!;
    // A choice the field would refuse stays in it, with the note, as a typed entry does.
    if (row.dataset["default"] !== "true" && this.#problemOf(target, value) !== undefined) {
      if (session.field !== undefined) {
        this.#setChoicesOpen(session, false);
        session.field.textContent = value;
        this.#selectAll(session.field);
      } else if (session.input !== undefined) {
        session.input.value = value;
      }
      this.#validate(session, true);
      return;
    }
    this.#endSession(session, true);
    session.host?.replaceChildren(...session.restore!);
    if (row.dataset["default"] === "true") {
      this.#resetValue(target);
      return;
    }
    this.#commitValue(target, value);
  }

  /** Leaves out `target`'s attribute, so its property reads its default, as Reset does. */
  #resetValue(target: EditTarget): void {
    const index = this.#index;
    const attribute = index?.tree.nodes[target.at]?.parent;
    if (target.adding === true || index === undefined || attribute === undefined || removalOf(index, attribute) !== "reset") {
      return;
    }
    const key = index.tree.nodes[attribute]!.key;
    if (this.#removed.has(key)) {
      return;
    }
    this.#removed.add(key);
    (this.#rendered.get(key) ?? this.#rendered.get(target.key))?.classList.add("pending");
    this.#fire(NX_EDIT_EVENT, { key, edit: { kind: "remove" } });
  }

  // ---- Steppers -----------------------------------------------------------------------------

  /**
   * Draws the tools past the value `span` draws for `target`: a number's steppers,
   * and, while its field is open with choices, their chevron. Nothing for a value with neither.
   */
  #showTools(span: HTMLElement, target: EditTarget): void {
    const session = this.#session?.target.key === target.key ? this.#session : undefined;
    const steppers = target.kind === "number";
    // The chevron says a list is there: past an open field with choices, and past a value with
    // choices under the pointer.
    const chevron = session === undefined ? this.#hasChoices(target) : session.field !== undefined && (session.choices?.length ?? 0) > 0;
    if (!steppers && !chevron) {
      this.#hideTools();
      return;
    }
    this.#tools.classList.toggle("with-steppers", steppers);
    this.#tools.classList.toggle("with-choices", chevron);
    this.#tools.classList.toggle("open", session?.choicesOpen === true);
    // With more of the line after it, an open field's tools take room in the line, as the field does
    // as it grows, rather than cover what follows.
    const wasInLine = this.#tools.isConnected && this.#tools.classList.contains("in-line");
    const inLine = session?.field !== undefined && this.#followedOnItsLine(span);
    this.#tools.classList.toggle("in-line", inLine);
    this.#tools.dataset["toolsFor"] = target.key;
    span.after(this.#tools);
    this.#toolsHover = undefined;
    if (inLine || wasInLine) {
      // The line grew or shrank, so its highlight is redrawn to fit it.
      queueMicrotask(() => this.#paintHighlights());
    }
  }

  /** Whether `target` offers choices its editor lists. */
  #hasChoices(target: EditTarget): boolean {
    return (target.kind === "number" || target.kind === "text") && choicesOf(target.hints, target.kind).length > 0;
  }

  /**
   * Whether more of its line follows the value `span` draws, within its one-line element: another
   * property or item, and not just the element's closing mark.
   */
  #followedOnItsLine(span: HTMLElement): boolean {
    for (let at: Element | null = span; at !== null && !at.matches(".line-element, .sheet-head, .sheet-value, .source"); at = at.parentElement) {
      for (let next = at.nextSibling; next !== null; next = next.nextSibling) {
        if (next.nodeType === 3) {
          const text = next.textContent ?? "";
          if (text.includes("\n")) {
            return false;
          }
          if (text.trim() !== "") {
            return true;
          }
        } else if (next.nodeType === 1 && !(next as Element).matches(".value-tools, .line-close")) {
          return true;
        }
      }
    }
    return false;
  }

  #hideTools(): void {
    const wasInLine = this.#tools.isConnected && this.#tools.classList.contains("in-line");
    this.#tools.remove();
    this.#tools.classList.remove("open", "in-line");
    if (wasInLine) {
      queueMicrotask(() => this.#paintHighlights());
    }
    delete this.#tools.dataset["toolsFor"];
    this.#toolsHover = undefined;
  }

  /**
   * Shows a number's steppers while the pointer rests on its value in the selected row, with no
   * editor open, and hides them once it leaves for anything but them.
   */
  #hoverTools(element: Element | null): void {
    if (!this.#editable || this.#session !== undefined || this.#stepping !== undefined) {
      return;
    }
    if (element !== null && this.#tools.contains(element)) {
      return;
    }
    // On the way from the value to the tools, the pointer crosses the space around it, which keeps
    // them; the gap past the value is the tools' own.
    const shown = this.#toolsHover === undefined || !this.#tools.isConnected ? undefined : this.#rendered.get(this.#toolsHover);
    if (shown !== undefined && element !== null && this.#onTheWay(shown, element)) {
      return;
    }
    const value = element?.closest?.(".editable-value:is(.edit-number, .edit-text)") as HTMLElement | null | undefined;
    const key = value === null || value === undefined ? undefined : keyOf(value);
    const target = key === undefined ? undefined : this.#targets.get(key);
    const selected = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
    // On the selected row: the row itself, a one-line element, or a property selected within one.
    const onSelected =
      value !== null &&
      value !== undefined &&
      selected?.contains(value) === true &&
      (this.#unitOf(value) === selected || !selected.matches(DRAWN_UNITS));
    // Under the pointer the tools only lie over the reading, so they show where they cover nothing.
    const offered = target !== undefined && (target.kind === "number" || this.#hasChoices(target));
    if (!onSelected || target === undefined || !offered || this.#rendered.get(key!) !== value || this.#followedOnItsLine(value!)) {
      if (this.#tools.isConnected) {
        this.#hideTools();
      }
      return;
    }
    if (this.#toolsHover === key && this.#tools.isConnected) {
      return;
    }
    this.#showTools(value!, target);
    this.#toolsHover = key;
  }

  /**
   * Whether `element` lies between the value `shown` and its tools: the value, or the row or line
   * that holds them, though not another value in it.
   */
  #onTheWay(shown: HTMLElement, element: Element): boolean {
    return (
      shown.contains(element) ||
      element === shown.parentElement ||
      element === shown.closest(".line-property, .sheet-row")
    );
  }

  #onToolsPointerDown(event: PointerEvent): void {
    event.stopPropagation();
    // The open field keeps the focus, and nothing is selected as text.
    event.preventDefault();
    const part = (event.target as Element).closest?.(".stepper, .choices-toggle") as HTMLElement | null;
    const key = this.#tools.dataset["toolsFor"];
    const target = key === undefined ? undefined : this.#targets.get(key);
    if (part === null || target === undefined || event.button !== 0) {
      return;
    }
    const session = this.#session?.target.key === key ? this.#session : undefined;
    if (part.classList.contains("choices-toggle")) {
      if (session !== undefined) {
        this.#toggleChoices(session, session.choicesOpen !== true);
        return;
      }
      // From the pointer's rest: the value's field opens with its list.
      this.#openEditor(key!);
      const opened = this.#session;
      if (opened !== undefined && opened.target.key === key) {
        this.#toggleChoices(opened, true);
      }
      return;
    }
    const from = session?.field !== undefined ? (session.field.textContent ?? "").trim() : this.#currentOf(target);
    this.#stepping = { target, pointer: event.pointerId, y: event.clientY, direction: part.classList.contains("up") ? 1 : -1, from, value: undefined };
    try {
      part.setPointerCapture?.(event.pointerId);
    } catch {
      // A pointer the element cannot capture still steps on release over it.
    }
  }

  #onToolsPointerMove(event: PointerEvent): void {
    const stepping = this.#stepping;
    if (stepping === undefined || event.pointerId !== stepping.pointer) {
      return;
    }
    event.stopPropagation();
    // Up is more, as the steppers point.
    const moved = stepping.y - event.clientY;
    if (stepping.value === undefined) {
      if (Math.abs(moved) < STEPPER_SLOP) {
        return;
      }
      // The drag counts from here, so it starts at the number pressed rather than a few steps on.
      stepping.y = event.clientY;
    }
    const target = stepping.target;
    const steps = Math.trunc((stepping.y - event.clientY) / STEPPER_STEP) * (event.shiftKey ? 10 : 1);
    const value = stepNumber(stepping.from, steps, target.signed !== false, target.hints) ?? stepping.from;
    if (stepping.value === undefined) {
      this.#body.classList.add("stepping");
    }
    stepping.value = value;
    this.#showStepped(target, value);
  }

  /**
   * Ends a press on a stepper: a click steps once, or ten steps with Shift; a drag has scrubbed. Into
   * the open field, or committed once, at release; a cancelled drag puts the value back.
   */
  #onToolsPointerUp(event: PointerEvent, commit: boolean): void {
    const stepping = this.#stepping;
    if (stepping === undefined || event.pointerId !== stepping.pointer) {
      return;
    }
    event.stopPropagation();
    this.#stepping = undefined;
    this.#body.classList.remove("stepping");
    const target = stepping.target;
    const dragged = stepping.value !== undefined;
    const value = !commit
      ? stepping.from
      : (stepping.value ??
        stepNumber(stepping.from, stepping.direction * (event.shiftKey ? 10 : 1), target.signed !== false, target.hints));
    if (value === undefined || (!dragged && value === stepping.from)) {
      return;
    }
    const session = this.#session?.target.key === target.key && this.#session.field !== undefined ? this.#session : undefined;
    if (session !== undefined) {
      this.#showStepped(target, value);
      if (commit) {
        this.#selectAll(session.field!);
      }
      return;
    }
    this.#endNumberDrag(target, stepping.from, value, commit);
  }

  /**
   * Carries a press on a stepper on through a reading drawn again: its tools drawn past the same
   * number, showing the number it has reached. When the number is gone, the press ends.
   */
  #resumeStepping(stepping: Stepping): void {
    const target = this.#targets.get(stepping.target.key);
    const session = this.#session?.target.key === stepping.target.key ? this.#session : undefined;
    const span = session?.span ?? this.#rendered.get(stepping.target.key);
    if (target === undefined || target.kind !== "number" || span === undefined) {
      return;
    }
    stepping.target = target;
    this.#stepping = stepping;
    this.#showTools(span, target);
    try {
      this.#tools.querySelector(stepping.direction === 1 ? ".stepper.up" : ".stepper.down")?.setPointerCapture?.(stepping.pointer);
    } catch {
      // A pointer the DOM no longer knows ends with its next event.
    }
    if (stepping.value !== undefined) {
      this.#body.classList.add("stepping");
      this.#showStepped(target, stepping.value);
    }
  }

  /**
   * Ends a drag that has moved `target`'s number from `from` to `value`, its field closed: committed
   * once, unless it was cancelled or is back where it began; then shown as before.
   */
  #endNumberDrag(target: EditTarget, from: string, value: string, commit: boolean): void {
    if (commit && value !== from && this.#problemOf(target, value) === undefined) {
      this.#commitValue(target, value);
      return;
    }
    const span = this.#rendered.get(target.key);
    if (span === undefined) {
      return;
    }
    if (this.#pending.has(target.key)) {
      showValue(span, target, this.#writtenOf(target), from);
    } else {
      this.#render();
    }
  }

  /** Shows `value` as `target`'s number while it steps: in its open field, or in its place, muted. */
  #showStepped(target: EditTarget, value: string): void {
    const session = this.#session;
    if (session?.target.key === target.key && session.field !== undefined) {
      session.field.textContent = value;
      this.#validate(session);
      return;
    }
    const span = this.#rendered.get(target.key);
    if (span !== undefined) {
      showValue(span, target, this.#writtenOf(target), value);
    }
  }

  // ---- Line breaks --------------------------------------------------------------------------

  /** Breaks the line where the caret is in `session`'s field, replacing what is selected. */
  #insertLineBreak(session: EditSession): void {
    const field = session.field!;
    const before = field.textContent ?? "";
    // The browser's own insertion keeps its undo; where it inserts nothing, the text is spliced.
    let inserted = false;
    try {
      inserted = this.ownerDocument.execCommand?.("insertText", false, "\n") === true && field.textContent !== before;
    } catch {
      inserted = false;
    }
    if (!inserted) {
      const caret = this.#caretOf(session) ?? { start: before.length, end: before.length };
      field.textContent = `${before.slice(0, caret.start)}\n${before.slice(caret.end)}`;
      this.#placeCaret(field, { start: caret.start + 1, end: caret.start + 1 });
    }
    this.#validate(session);
  }

  // ---- Properties an element does not write -------------------------------------------------

  /**
   * Draws, on the selected element, the quiet row that lists the properties it does not write, and
   * under it, when open, a dimmed row to each. Drawn again only when the element or its state
   * changed, or with `force`.
   */
  #drawMore(force = false): void {
    const key = this.#selection;
    const open = key !== undefined && this.#moreOpen === key;
    const state = key === undefined ? undefined : `${key}\u0000${open}`;
    if (!force && state === this.#moreFor && (state === undefined || this.#moreDrawn.some((drawn) => this.#body.contains(drawn)))) {
      return;
    }
    const focused = this.shadowRoot?.activeElement;
    const refocus = focused instanceof HTMLElement && this.#moreDrawn.some((drawn) => drawn.contains(focused)) ? focused.dataset["moreFocus"] : undefined;
    for (const drawn of this.#moreDrawn) {
      drawn.remove();
    }
    for (const stale of this.#moreKeys) {
      this.#targets.delete(stale);
      this.#rendered.delete(stale);
    }
    this.#moreDrawn = [];
    this.#moreBlock = undefined;
    this.#moreKeys = [];
    this.#moreCards.clear();
    this.#moreFor = state;
    const index = this.#index;
    const at = key === undefined ? undefined : index?.byKey.get(key);
    const element = key === undefined ? undefined : this.#rendered.get(key);
    if (!this.#editable || index === undefined || at === undefined || element === undefined || !element.matches(".sheet:not(.folded), .line-element, .tag-line")) {
      return;
    }
    const unwritten = unwrittenProperties(index, at);
    if (unwritten.length === 0) {
      return;
    }
    const document = this.ownerDocument;
    const rows: HTMLElement[] = [];
    const toggle = document.createElement("span");
    toggle.className = open ? "sheet-row more more-toggle open" : "sheet-row more more-toggle";
    toggle.tabIndex = 0;
    toggle.dataset["moreFocus"] = "toggle";
    toggle.setAttribute("role", "button");
    toggle.setAttribute("aria-expanded", String(open));
    const label = document.createElement("span");
    label.className = "more-label";
    const properties = unwritten.length === 1 ? "property" : "properties";
    // Named for what it lists, unset properties, so it does not read as more of those written.
    label.textContent = open ? `Hide unset ${properties}` : `Show ${unwritten.length} unset ${properties}`;
    toggle.setAttribute("aria-label", `${unwritten.length} unset ${properties}`);
    toggle.append(label);
    rows.push(toggle);
    if (open) {
      if (unwritten.length > MORE_FILTER_PAST) {
        const holder = document.createElement("span");
        holder.className = "sheet-row more more-filter-row";
        const filter = document.createElement("input");
        filter.className = "more-filter";
        filter.placeholder = "Filter properties";
        filter.spellcheck = false;
        filter.value = this.#moreFilter;
        filter.dataset["moreFocus"] = "filter";
        filter.setAttribute("aria-label", "Filter properties");
        filter.addEventListener("input", () => {
          this.#moreFilter = filter.value;
          this.#filterMore();
        });
        holder.append(filter);
        rows.push(holder);
      }
      for (const entry of unwritten) {
        rows.push(this.#dimmedRow(index, at, key!, entry));
      }
    }
    for (const row of rows) {
      row.dataset["moreOf"] = key;
    }
    if (element.matches(".sheet")) {
      const written = [...element.children].filter((child) => child.matches(".sheet-row:not(.more), .sheet-head"));
      written[written.length - 1]!.after(...rows);
      this.#moreDrawn = rows;
    } else {
      // A line lists them under it, as a group would.
      const block = document.createElement("span");
      block.className = "more-block";
      block.dataset["moreOf"] = key;
      const inGroup = element.parentElement?.closest(".sheet") !== null;
      const line = this.#text.split("\n")[index.tree.nodes[at]!.range.start.line] ?? "";
      block.style.setProperty("--more-indent", String(inGroup ? 0 : line.length - line.trimStart().length));
      block.append(...rows);
      // After the line's end, so the block takes no blank line of its own.
      const next = element.nextSibling;
      if (next !== null && next.nodeType === 3 && (next.textContent ?? "").startsWith("\n")) {
        (next as Text).splitText(1);
        next.after(block);
      } else {
        element.after(block);
      }
      // Under the element itself, inside its highlight, where it starts partway along its line.
      const blockBox = block.getBoundingClientRect();
      const from = element.getClientRects()[0];
      if (blockBox.width > 0 && from !== undefined) {
        // In a narrow pane, no further in than halfway, so the rows keep room to read.
        block.style.paddingLeft = `${Math.min(blockBox.width / 2, Math.max(0, from.left - blockBox.left))}px`;
      }
      this.#moreDrawn = [block];
      this.#moreBlock = block;
    }
    for (const value of this.#moreKeys) {
      const span = this.#rendered.get(value);
      const pending = this.#pending.get(value);
      if (span !== undefined && pending !== undefined) {
        showValue(span, this.#targets.get(value)!, "", pending);
        this.#markAdded(this.#targets.get(value)!, span);
      }
    }
    this.#filterMore();
    if (refocus !== undefined) {
      [...this.#moreDrawn.flatMap((drawn) => [drawn, ...drawn.querySelectorAll<HTMLElement>("[data-more-focus]")])]
        .find((candidate) => candidate.dataset["moreFocus"] === refocus)
        ?.focus({ preventScroll: true });
    }
    queueMicrotask(() => this.#paintHighlights());
  }

  /** A dimmed row for a property the element at `at`, with `key`, does not write. */
  #dimmedRow(index: TreeIndex, at: number, key: string, entry: UnwrittenProperty): HTMLElement {
    const document = this.ownerDocument;
    const { property, editor, hints } = entry;
    const attributeKey = `${key}.${property.name}`;
    const valueKey = `${attributeKey}.value`;
    const row = document.createElement("span");
    row.className = "sheet-row more dimmed-row";
    row.tabIndex = -1;
    row.dataset["moreFocus"] = `row:${property.name}`;
    row.dataset["name"] = property.name;
    const name = document.createElement("span");
    name.className = "sheet-name";
    name.textContent = property.name;
    name.dataset["hover"] = attributeKey;
    this.#moreCards.set(attributeKey, propertyCardOf(property));
    const cell = document.createElement("span");
    cell.className = "sheet-value";
    const value = document.createElement("span");
    value.className = "more-value";
    // With no default to show, the value's place reads the type it takes, quieter.
    if (property.default === undefined) {
      value.dataset["placeholder"] = hints?.placeholder ?? property.type;
    }
    const start = defaultValue(property.default, editor);
    value.append(...defaultShown(document, property.default, editor, entry.format));
    cell.append(value);
    if (editor !== undefined) {
      const target: EditTarget = {
        at,
        key: valueKey,
        kind: editor.kind,
        value: start,
        ...(editor.cases === undefined ? {} : { cases: editor.cases }),
        signed: true,
        ...(hints === undefined ? {} : { hints }),
        property: { element: key, name: property.name },
        adding: true
      };
      value.dataset["key"] = valueKey;
      value.classList.add("editable-value", `edit-${editor.kind}`);
      this.#targets.set(valueKey, target);
      this.#rendered.set(valueKey, value);
      this.#moreKeys.push(valueKey);
      if (editor.kind === "number") {
        name.classList.add("scrub");
        name.dataset["scrubKey"] = valueKey;
      }
    } else {
      // No editor takes its type, as a record's: Enter or a double-click opens the element in Source.
      row.dataset["opens"] = "true";
    }
    if (property.default !== undefined) {
      const quiet = document.createElement("span");
      quiet.className = "more-default";
      quiet.textContent = "default";
      cell.append(" ", quiet);
    }
    row.append(name, cell);
    row.setAttribute("aria-label", `${property.name}, not written${property.default === undefined ? "" : `, default ${property.default}`}`);
    return row;
  }

  /** Hides the dimmed rows whose names do not hold what the filter holds. */
  #filterMore(): void {
    const query = this.#moreFilter.trim().toLowerCase();
    for (const drawn of this.#moreDrawn) {
      const rows = drawn.matches(".dimmed-row, .added-row") ? [drawn] : [...drawn.querySelectorAll<HTMLElement>(".dimmed-row, .added-row")];
      for (const row of rows) {
        row.hidden = query !== "" && !row.dataset["name"]!.toLowerCase().includes(query);
      }
    }
  }

  /** Opens or closes the selected element's list of unwritten properties. */
  #toggleMore(): void {
    const key = this.#selection;
    if (key === undefined) {
      return;
    }
    this.#moreOpen = this.#moreOpen === key ? undefined : key;
    this.#moreFilter = "";
    this.#drawMore();
    const focus = this.#moreDrawn
      .flatMap((drawn) => [drawn, ...drawn.querySelectorAll<HTMLElement>("[data-more-focus]")])
      .find((candidate) => candidate.dataset["moreFocus"] === (this.#moreOpen === undefined ? "toggle" : "filter"));
    if (this.#moreOpen !== undefined && focus !== undefined) {
      focus.focus({ preventScroll: true });
    } else if (this.shadowRoot?.activeElement === null || this.shadowRoot?.activeElement === undefined) {
      this.#body.focus({ preventScroll: true });
    }
  }

  /** Closes the list of unwritten properties, and the reading takes the keys again. */
  #closeMore(): void {
    this.#moreOpen = undefined;
    this.#moreFilter = "";
    this.#drawMore();
    this.#body.focus({ preventScroll: true });
  }

  /** The keys of the list of unwritten properties, where focus is in it. */
  #onMoreKey(event: KeyboardEvent, target: HTMLElement): void {
    const focusables = this.#moreDrawn
      .flatMap((drawn) => [drawn, ...drawn.querySelectorAll<HTMLElement>("[data-more-focus]")])
      .filter((candidate) => candidate.dataset["moreFocus"] !== undefined && !candidate.hidden && candidate.closest("[hidden]") === null);
    const holder = target.closest<HTMLElement>("[data-more-focus]");
    const at = holder === null ? -1 : focusables.indexOf(holder);
    if (event.key === "Escape") {
      event.preventDefault();
      this.#closeMore();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const next = at + (event.key === "ArrowDown" ? 1 : -1);
      if (next < 0) {
        this.#body.focus({ preventScroll: true });
      } else {
        focusables[Math.min(next, focusables.length - 1)]?.focus({ preventScroll: true });
      }
      return;
    }
    if (holder?.classList.contains("more-toggle") === true && (event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      this.#toggleMore();
      return;
    }
    if (holder?.classList.contains("dimmed-row") !== true && holder?.classList.contains("added-row") !== true) {
      return;
    }
    const value = holder.querySelector<HTMLElement>(".editable-value");
    if (event.key === "Enter" || event.key === "F2" || (event.key === " " && value !== null && this.#targets.get(keyOf(value)!)?.kind === "boolean")) {
      event.preventDefault();
      if (value !== null) {
        this.#openEditor(keyOf(value)!);
      } else if (event.key === "Enter") {
        this.#openMoreInSource(holder);
      }
    }
  }

  /** Fires `nx-open` for the element a dimmed row with no editor is on, to set it in Source. */
  #openMoreInSource(row: HTMLElement): void {
    const at = this.#index?.byKey.get(row.dataset["moreOf"] ?? "");
    if (at !== undefined) {
      this.#fireOpen(at);
    }
  }

  // ---- A row's menu -------------------------------------------------------------------------

  /** Opens the menu of the written row with `key`, against `anchor`, selecting the row. */
  #openMenu(key: string, anchor: HTMLElement): void {
    const index = this.#index;
    const at = index?.byKey.get(key);
    if (!this.#editable || index === undefined || at === undefined) {
      return;
    }
    if (this.#session !== undefined) {
      this.#cancel(this.#session, false);
    }
    this.#closeCard();
    if (this.#selection !== key) {
      this.#select(key, true, false);
    }
    const document = this.ownerDocument;
    const items: HTMLElement[] = [];
    const item = (action: string, words: string, value: Node[] = []): void => {
      const button = document.createElement("div");
      button.className = "menu-item";
      button.tabIndex = -1;
      button.dataset["action"] = action;
      button.setAttribute("role", "menuitem");
      const label = document.createElement("span");
      label.className = "menu-label";
      label.textContent = words;
      button.append(label);
      if (value.length > 0) {
        // What the item leads to, quieter, past its words, as a choice's value is.
        const shown = document.createElement("span");
        shown.className = "menu-value";
        shown.append(...value);
        button.append(shown);
        button.setAttribute("aria-label", `${words}, ${shown.textContent?.trim() ?? ""}`);
      }
      items.push(button);
    };
    const removal = this.#removed.has(key) ? undefined : removalOf(index, at);
    if (removal === "reset") {
      item("remove", "Reset to default", this.#defaultShownFor(at));
    } else if (removal === "remove") {
      item("remove", "Remove");
    }
    item("source", "Show in Source");
    this.#menu.replaceChildren(...items);
    this.#menu.setAttribute("aria-label", `Actions for ${index.tree.nodes[at]!.name ?? "the row"}`);
    this.#menu.hidden = false;
    this.#menuKey = key;
    this.#placeCard(anchor, this.#menu);
    items[0]!.focus({ preventScroll: true });
  }

  /**
   * The default of the property the attribute at `at` sets, drawn as the reading draws its value, or
   * nothing when it is not a short literal, such as an element or a record.
   */
  #defaultShownFor(at: number): Node[] {
    const index = this.#index!;
    const attribute = index.tree.nodes[at]!;
    const element = attribute.parent === undefined ? undefined : index.tree.nodes[attribute.parent];
    const declared = element === undefined ? undefined : elementProperties(index.tree, element)?.find((candidate) => candidate.name === attribute.name);
    const written = declared?.default?.trim();
    if (written === undefined || written.length > 24 || /^[<{]/.test(written)) {
      return [];
    }
    const target = this.#targets.get(`${attribute.key}.value`);
    const editor = target === undefined || target.kind === "run" ? undefined : { kind: target.kind };
    return defaultShown(this.ownerDocument, written, editor, target?.kind === "color" ? "color" : undefined);
  }

  /** Closes the row's menu; with `refocus`, the reading takes the keys again. */
  #closeMenu(refocus: boolean): void {
    if (this.#menuKey === undefined) {
      return;
    }
    this.#menuKey = undefined;
    this.#menu.hidden = true;
    this.#menu.replaceChildren();
    if (refocus) {
      this.#body.focus({ preventScroll: true });
    }
  }

  #onMenuKey(event: KeyboardEvent): void {
    event.stopPropagation();
    const items = [...this.#menu.querySelectorAll<HTMLElement>(".menu-item")];
    const at = items.indexOf(event.target as HTMLElement);
    switch (event.key) {
      case "Escape":
      case "Tab":
        event.preventDefault();
        this.#closeMenu(true);
        return;
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const next = (at + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
        items[next]?.focus({ preventScroll: true });
        return;
      }
      case "Home":
      case "End":
        event.preventDefault();
        items[event.key === "Home" ? 0 : items.length - 1]?.focus({ preventScroll: true });
        return;
      case "Enter":
      case " ":
        event.preventDefault();
        if (at >= 0) {
          this.#chooseMenuItem(items[at]!);
        }
    }
  }

  /** Does what the menu's `item` says to its row, and closes the menu. */
  #chooseMenuItem(item: HTMLElement): void {
    const key = this.#menuKey;
    this.#closeMenu(true);
    if (key === undefined) {
      return;
    }
    if (this.#selection !== key) {
      this.#select(key, true, false);
    }
    if (item.dataset["action"] === "remove") {
      this.#removeSelected();
    } else {
      this.#openSelected(true, true);
    }
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
    const value = stepNumber(scrub.from, steps, scrub.target.signed !== false, scrub.target.hints) ?? scrub.from;
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
    this.#endNumberDrag(scrub.target, scrub.from, scrub.value, commit);
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
    // The press that ended a drag of a number's name, or a click inside an open field, is not a
    // choice of node.
    if (this.#ignoreClick) {
      this.#ignoreClick = false;
      return;
    }
    if (this.#session?.field?.contains(target) === true) {
      return;
    }
    // A row's menu opens from the mark at its end.
    const mark = target.closest?.(".row-menu") as HTMLElement | null | undefined;
    if (mark !== null && mark !== undefined && this.#body.contains(mark)) {
      const row = mark.closest<HTMLElement>(".sheet-row[data-key]")!;
      if (this.#menuKey === row.dataset["key"]) {
        this.#closeMenu(true);
      } else {
        this.#openMenu(row.dataset["key"]!, mark);
      }
      return;
    }
    // The quiet row on a selected element opens and closes its list of unwritten properties, and the
    // rest of that list, but its values, is no choice of node.
    const more = target.closest?.(".more") as HTMLElement | null | undefined;
    if (more !== null && more !== undefined && this.#body.contains(more)) {
      if (more.classList.contains("more-toggle")) {
        this.#toggleMore();
        return;
      }
      if (target.closest?.(".editable-value") == null) {
        // A click anywhere on a dimmed row edits its value, which may be empty, as an optional one is.
        const row = target.closest?.(".dimmed-row, .added-row") as HTMLElement | null | undefined;
        const value = row?.querySelector<HTMLElement>(".editable-value");
        if (value !== null && value !== undefined && target.closest?.(".scrub") == null) {
          if (this.#session?.target.key !== keyOf(value)) {
            this.#openEditor(keyOf(value)!);
          }
        } else if (!target.matches?.("input")) {
          (more.closest<HTMLElement>("[data-more-focus]") ?? more).focus({ preventScroll: true });
        }
        return;
      }
    }
    // A drag that selected text is a text selection, not a choice of node.
    // Chromium reports a selection inside a shadow root only through the root itself.
    const root = this.shadowRoot as (ShadowRoot & { getSelection?(): Selection | null }) | null;
    const textSelection = root?.getSelection?.() ?? this.ownerDocument.getSelection?.();
    if (textSelection !== null && textSelection !== undefined && !textSelection.isCollapsed) {
      return;
    }
    // A click on an editable value edits it, as a form's field does: a field opens with the caret
    // where the click fell, a boolean toggles, and a case or a color opens its popover. Its unit is
    // selected with it. A click anywhere else in a unit, as on a property's name, selects the unit.
    const value = this.#editable ? (target.closest?.(".editable-value") as HTMLElement | null | undefined) : undefined;
    if (value !== null && value !== undefined && this.#body.contains(value)) {
      const key = keyOf(value)!;
      // The second click of a double-click would toggle a boolean back, or open an editor again.
      const closed = this.#closedOnPress === key;
      this.#closedOnPress = undefined;
      if (closed || (event.detail > 1 && this.#targets.get(key)?.kind === "boolean") || this.#session?.target.key === key) {
        return;
      }
      const unit = keyOf(this.#unitOf(value) ?? value);
      if (unit !== this.#selection) {
        this.#select(unit, true, false);
      }
      this.#openEditor(key);
      const field = this.#session?.target.key === key ? this.#session.field : undefined;
      if (field !== undefined) {
        const at = offsetAtPoint(field, event.clientX, event.clientY);
        this.#placeCaret(field, { start: at, end: at });
      }
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
    const more = target.closest?.(".more, .more-block") as HTMLElement | null | undefined;
    if (more !== null && more !== undefined) {
      this.#onMoreKey(event, target as HTMLElement);
      return;
    }
    // The context menu key, or Shift+F10, opens the selected row's menu.
    if (this.#editable && (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey))) {
      const row = this.#selection === undefined ? undefined : this.#rendered.get(this.#selection);
      const mark = row?.matches(".sheet-row") === true ? row.querySelector<HTMLElement>(":scope > .row-menu") : null;
      if (mark !== null && mark !== undefined) {
        event.preventDefault();
        this.#openMenu(this.#selection!, mark);
      }
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
      if (this.#moreOpen !== undefined) {
        this.#closeMore();
      }
      return;
    }
    // Cmd+Backspace leaves out the selected row's property, as Delete does: a Mac laptop's delete
    // key is Backspace.
    if (this.#editable && event.key === "Backspace" && event.metaKey && !event.ctrlKey && !event.altKey) {
      if (this.#removeSelected()) {
        event.preventDefault();
      }
      return;
    }
    // Enter or F2 opens the selected unit's first value, Space toggles its boolean, and Delete leaves
    // out its property. Ctrl or Cmd with Z or Y are the host's, for its undo.
    if (this.#editable && !event.ctrlKey && !event.metaKey && !event.altKey) {
      if (event.key === "Enter" || event.key === "F2") {
        if (this.#openSelected(event.key === "Enter")) {
          event.preventDefault();
        } else if (event.key === "Enter" && this.#moreDrawn.length > 0) {
          // An element with no value of its own to open opens its list of unwritten properties.
          event.preventDefault();
          this.#toggleMore();
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
      // Not Backspace, which a reader who meant to change the text would press.
      if (event.key === "Delete") {
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
    const card = index === undefined || at === undefined ? this.#moreCards.get(key) : cardOf(index, at);
    if (card === undefined || !this.#body.contains(anchor)) {
      return undefined;
    }
    clearTimeout(this.#cardCloseTimer);
    this.#fillCard(card);
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

  /** Puts `card`'s signature and doc in the hover card. */
  #fillCard(card: Card): void {
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
