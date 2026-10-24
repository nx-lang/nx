# @nx-lang/viewer

An `<nx-viewer>` custom element that shows NX source as a reading: the document's own text, with each
element read the way a design tool's layers read rather than as tags. An element whose properties
fit on one line reads as its name and properties; one whose properties do not, or that holds
elements, reads as a group under its name, with no angle brackets or closing tag; values lose their
quotes; and the rest reads as written, in the colors of the host's editor. It has no framework dependency and keeps its
markup and styles in a shadow root, so it drops into a plain page, a React app, a docs site or a VS
Code webview alike.

What it shows is the source tree the NX language service answers for a document: `sourceTree(uri)`
on a language snapshot of [`@nx-lang/sdk-wasm`](../../bindings/wasm/README.md), or the same query
through any `NxLanguageService`. With nothing folded, the reading keeps every node: each node of the
tree is rendered once and carries its key, so a host can move between the reading and its editor.
What is folded is not rendered, and selecting a node folded away opens what hides it.

## A plain page

```html
<nx-viewer id="reading"></nx-viewer>

<script type="module">
  import "@nx-lang/viewer";
  import { createNxHost, compileNxModule } from "@nx-lang/sdk-wasm";
  // With a bundler such as Vite; otherwise, wherever the page serves the package's `nx.wasm`.
  import nxModuleUrl from "@nx-lang/sdk-wasm/nx.wasm?url";

  const text = `type User = { id:string name:string? active:bool = true }

let users = {
  <User id="1" name="Ada" />
}`;
  const host = createNxHost(await compileNxModule(fetch(nxModuleUrl)));
  const snapshot = host.createLanguageSnapshot([{ uri: "nx:///user.nx", source: text }]);
  const viewer = document.getElementById("reading");
  try {
    viewer.text = text;
    viewer.tree = snapshot.sourceTree("nx:///user.nx", { visualizers: {} });
  } finally {
    snapshot.dispose();
  }
</script>
```

The type reads as written, and the element reads `User  id=1  name=Ada`.

Importing the package defines the element. Where there is no DOM, as in a server render, importing
it does nothing, and `defineNxViewerElement(registry)` defines it in a registry of your choosing.

## React

React 19 passes properties to custom elements directly, so no wrapper is needed:

```tsx
import { useEffect, useRef } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement } from "@nx-lang/viewer";
import type { SourceTree } from "@nx-lang/language-protocol";

export function Reading(props: { tree: SourceTree; text: string; onSelect: (key: string) => void }) {
  const viewer = useRef<NxViewerElement>(null);
  useEffect(() => {
    const element = viewer.current!;
    const listener = (event: Event) => props.onSelect((event as CustomEvent<NxSelectDetail>).detail.key);
    element.addEventListener(NX_SELECT_EVENT, listener);
    return () => element.removeEventListener(NX_SELECT_EVENT, listener);
  }, [props.onSelect]);
  return <nx-viewer ref={viewer} text={props.text} tree={props.tree} />;
}
```

Set `text` before `tree`, as above: each renders, and the tree is read against the text. Declare the
element for JSX once in your app:

```ts
import type { NxViewerElement } from "@nx-lang/viewer";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "nx-viewer": React.DetailedHTMLProps<React.HTMLAttributes<NxViewerElement>, NxViewerElement> &
        Partial<Pick<NxViewerElement, "tree" | "text" | "colors" | "icons" | "folded" | "stale" | "selection">>;
    }
  }
}
```

## Properties and attributes

| Name        | Kind                   | Meaning                                                                      |
| ----------- | ---------------------- | ---------------------------------------------------------------------------- |
| `tree`      | property               | The `SourceTree` to show                                                     |
| `text`      | property               | The document's text, which the tree's byte ranges index                      |
| `colors`    | property               | The text's colors, as ranges over `text`, from the host's highlighter        |
| `icons`     | property               | Draws an icon by its Lucide name, for each element that starts a line of its own; no icons without it |
| `markdown`  | property               | Renders a doc comment's Markdown for the hover cards, as a `Node`; a built-in renderer without it |
| `folded`    | property               | The keys of what is folded; `undefined`, the default, folds the document's supporting declarations |
| `stale`     | property and attribute | The reading is muted and badged "Out of date"; selection still works         |
| `selection` | property               | The selected node's key; setting it scrolls the node into view               |
| `editable`  | property               | Values edit in place, and edits fire `nx-edit`; off by default               |

## How it reads

How an element reads depends on the pane's width, not on where the source breaks its lines. The
element measures what fits as it draws, and again once a change in the pane's width settles (not on
every step of a dragged divider) or its fonts load, keeping what is at the top where it was.

- **An element whose properties fit on one line** reads as a line: its icon and name, its title's
  value alone, each other attribute as `name=value`, and any text it holds. So does one with a single
  property, which wraps if it must.
- **An element that holds elements, or whose properties do not fit on one line,** reads as a group:
  a header with its icon, its name in bold and its id; its properties on the header as `name=value`
  when they fit there (with `idPosition=end`, the id follows them), or else a row for each, names in
  one quiet column and values in another; then what it holds, indented. A property whose value needs
  lines of its own, such as a list, has a row either way. There are no angle brackets and no closing
  tag. A bracket runs down the group from under its name and turns right at its end. A value of
  several lines reads under its name, and so does every value in a pane narrower than about 64
  characters.
- **A property's element value** (`Padding=<Thickness …/>`) reads on its row, as a line, when it fits
  there, and otherwise, or when it holds elements, as a group under the property's name.
- **A line that shares its line**, with a property's name or a `let`, keeps a quiet `<` and `/>` to
  show where it starts and ends. Alone on its line, in a place typed exactly as its kind (a `Choice`
  in `choices: Choice+`), it leaves out its name, which its icon carries.
- **What a group holds** reads an item to a line. An `if` or a `for` block reads as written, its
  later lines indented from its own start, and so does an element a group cannot show, such as one
  with a comment inside an attribute. A blank line between two items, and the space between two
  siblings where either is a group, becomes a little room.
- **Values**: strings lose their quotes, and booleans read ✓ and ✗. A string typed `uitext` (an alias
  `type uitext = string`, or any alias that leads to it) reads as text shown to people, in the prose
  font. A string a visualizer names as an element's `id`, or formats as `id`, reads as one quiet token; a
  string formatted as `color` reads with a swatch.
- **Everything else** (declarations, function bodies, comments) reads as written, in `colors`.
- **Long lines** wrap a step in from where their unit starts: a row's value, a line with a single
  property, and an item a group holds as written. Code read as written
  elsewhere keeps its lines.

## Visualizer declarations

Ask the language service for the tree with `visualizers`, and the declaration table carries each
declaration's resolved visualizer, from the `visualizer` declarations of the document's program and of any
host modules named:

```ts
viewer.tree = snapshot.sourceTree("nx:///survey/form.nx", { visualizers: { modules: ["tenant/visualizers.nx"] } });
```

The reading uses an element visualizer's `title` (its value reads alone on the element's line), `id` and
`idPosition` (the id reads as a quiet token after the name, or last on the line), and `icon` (the
Lucide icon the element starts with; a kind inherits it), a value's `format` of `id` or `color`, and
each property's editor hints (see [Editing](#editing)). The order of an element visualizer's
`properties` is the order in which a selected element lists the properties it does not write.

## Icons

The element draws no icons of its own. Give it a function that draws one by its Lucide name, and every
element that starts a line of its own starts with an icon: the one its visualizer's `icon` names, or, when
there is none or the function draws nothing for that name, one for its kind (`component`, `box` for a record, `zap` for an action, `tag` for a union case,
`square-function` for a function). The playground loads Lucide apart from the rest of its code:

```ts
import { icons, createElement } from "lucide";

viewer.icons = (name) => {
  const node = icons[name.replace(/(^|-)([a-z0-9])/g, (_, _dash, letter) => letter.toUpperCase())];
  return node === undefined ? undefined : createElement(node);
};
```

## Folding

A group folds to its header, followed by its title or a count of what it hides, such as
`7 properties · 1 child`, and a declaration written over several lines folds to its first line. The
children are counted as the group shows them open: each element, and each expression that gives
elements, such as `{Children}`, once; inside an `if`, a match or a `for`, what every branch or its
body writes. Text and comments are not children. A run of declarations that support the document
(imports, types, actions, components with no body of their own, external or abstract, visualizer
declarations, and functions other than `root` and element functions) folds to one line that says what it holds, such as `8 types · 13
components · 2 visualizers`. Until the host or the reader folds anything, those runs and their
declarations start folded, so a document opens on its own content. `if` and `for` blocks don't fold.

A chevron hangs left of each unit that folds. It shows while the pointer is over the reading, and
always on what is folded. A click folds or unfolds; an Alt+click does the same to everything inside.
With a group selected, the left arrow folds it and the right arrow opens it. Selecting something
folded away opens what hides it, and folding what holds the selection keeps it folded and selects it (a run, which is not a node,
clears the selection instead). `collapseAll()` and `expandAll()` fold and open everything, and each
change fires `nx-fold`, whose `detail.folded` lists the keys folded and `detail.selection` the selection after the fold. A host keeps them in `folded`
to show the document as the reader left it.

## Selection

A click selects the unit it falls in: a group (by its header, or the room between its rows), a row, an
element on one line, or a block such as an `if`. With `editable` set, a click on a value opens its
editor instead, and leaves the selection as it was. Outside every unit, as in a function's body, it
selects the smallest node under the pointer. Selecting fires `nx-select`, whose `detail` is
`{ key, role, range }`; `range` carries both editor positions and UTF-8 byte offsets, so a host can
put an editor's cursor at the node. Setting `selection` selects without firing the event, and a key
the tree does not hold clears it.

The selected unit and the one under the pointer are drawn as full-width rows behind the text, a
selected group's first line stronger than the rest of it. From the keyboard, the reading takes focus,
the up and down arrows walk the units, the left arrow selects the group holding the selected unit, and
the right arrow the first unit it holds. A selection that is not a unit, such as a node the host
selects inside a row, moves from the unit that holds it. The selected node carries `aria-current`, and a polite live
region says what was selected.

`nodeAtSpan(tree, startByte, endByte)` finds the node whose range is exactly that span, preferring an
element. A value origin from the IR runtime carries such a span, so a host can select the source of
a value it shows.

Two helpers carry a selection between the reading and a host's editor, both in UTF-16 offsets into
the text the tree was computed from, the way JavaScript strings and Monaco's `getOffsetAt` count:

- `keyAtRange(tree, text, start, end?)` is the key of the smallest node holding the text from
  `start` to `end`, less the whitespace at either end, so a selection of whole lines finds what they
  hold; without `end`, the node at a cursor. It is `undefined` between nodes. Set it as `selection`
  when the reading is shown.
- `rangeOfKey(tree, text, key)` is where that node starts and ends. Select it in the editor when the
  editor is shown again.

Three methods of the element carry the place, so the view switched to shows the same part of the
document. Each measures from the top of what scrolls the reading, its nearest scrolling ancestor or
the page, which must be laid out (hide it with `visibility: hidden`, not `display: none`, to measure
it while the editor shows):

- `keyAtTop()` is the first node drawn at or below that top, as `{ key, top }`, `top` in pixels.
- `topOf(key)` is how far below that top the node is drawn, or the nearest drawn node holding it,
  as a folded group.
- `scrollToKey(key, top)` scrolls the reading so that node is drawn `top` pixels below it.

```ts
import { keyAtRange, rangeOfKey } from "@nx-lang/viewer";

// Into the reading: the editor's selection, and the line at the editor's top at the same height.
const selection = editor.getSelection();
viewer.selection = keyAtRange(tree, text, model.getOffsetAt(selection.getStartPosition()), model.getOffsetAt(selection.getEndPosition()));
const line = editor.getVisibleRanges()[0].startLineNumber;
const key = keyAtRange(tree, text, model.getOffsetAt({ lineNumber: line, column: model.getLineFirstNonWhitespaceColumn(line) }));
if (key !== undefined) viewer.scrollToKey(key, editor.getTopForLineNumber(line) - editor.getScrollTop());

// ...and back: the node's text, and the node at the reading's top at the same height.
const range = viewer.selection === undefined ? undefined : rangeOfKey(tree, text, viewer.selection);
if (range !== undefined) editor.setSelection(monaco.Selection.fromPositions(model.getPositionAt(range.start), model.getPositionAt(range.end)));
const atTop = viewer.keyAtTop();
const start = atTop === undefined ? undefined : rangeOfKey(tree, text, atTop.key)?.start;
if (atTop !== undefined && start !== undefined) {
  editor.setScrollTop(editor.getTopForLineNumber(model.getPositionAt(start).lineNumber) - atTop.top);
}
```

## Editing

With `editable` set, a reader changes a value where it reads, with an editor that fits its type.
The element never changes the text or the tree itself: each edit fires `nx-edit`, whose `detail` is
`{ key, edit }`, the node's key and the edit as the language service's `sourceEdit` query takes it.
The host asks its language service for the text edits, applies them to its own text, so the edit
joins its editor's undo history, and sets the new text and tree. Until it does, the edited value
reads muted; a host that sets the same tree again, as after a refused edit, has it read as before.

| Value                                              | Editor                                                        |
| -------------------------------------------------- | ------------------------------------------------------------- |
| A string, `uitext` or an id                        | A field in the value's place, holding it without quotes or escapes |
| An `int`, `float32` or `float64`, or an alias of one | A number field: the up and down arrows step by 1, Shift by 10; dragging the property's name sideways changes it, committed once on release |
| A boolean                                          | Toggles: a click on its mark, or Space                         |
| A bare case of a union                             | A popover of the union's cases that hold no properties, each with its doc, filtered as the reader types |
| A string formatted as `color`                      | A popover with a hex field and the browser's color picker      |
| A run of text on one line                          | A one-line field                                              |

Editor hints from the tree's visualizers shape the editors:

| Hint                                         | Editor                                                     |
| -------------------------------------------- | ---------------------------------------------------------- |
| `editor=field`                               | The number field every number has                          |
| `min`, `max`, `step`                         | Arrows, steppers and dragging stay in the range and step by `step`; a typed number outside it stays in the field with a note, as `At most 64` |
| `ValueChoice` content (`choices`)            | A list under a number's or a string's field: each choice's label, then its value, quieter, or the value alone when none has a label; the default's row, `Default 16`, leads it |
| `editor=segmented`                           | A case's popover shows the cases as one row of buttons, or the list past five cases or when they do not fit |
| `multiline`                                  | Enter breaks the line, and Ctrl or Cmd+Enter commits      |
| `placeholder`                                | Shown, quieter, in the empty field; never written         |

In any string's field Shift+Enter breaks the line, and a string written over several lines edits too.

A field with choices shows a chevron just past it, and so does the value while the pointer rests on
it; a click on that chevron opens the field with its list. A click on the
chevron, Alt+Down, or typing lists them, typing keeping those whose label or value holds what was
typed; the current value's row is marked. While the list is closed, Up and Down step a number; while it is open, they move through it, from the current value's row,
Enter picks the active row, and Escape closes the list before the field. Picking a choice commits its
value, never its label; picking the default's row fires `{ kind: "remove" }` for the attribute, as
Reset to default does, or, on a dimmed row, adds nothing. The field stays typeable: a choice is a
suggestion, and any entry the field takes commits. A choice the field would refuse, as one past
`max`, stays in the field with the field's note. A color's choices lead its popover, over its hex
field.

A number shows steppers, an up and a down button drawn just past the value so nothing moves, while
its field is open and while the pointer rests on its value, selected or not. Where they take room in
a line, outside the selection, they wait until the pointer has rested on the value for 300 ms, so a
line doesn't jump as the pointer passes over it. A click steps once, Shift+click ten steps, within the range: into the open field, or committed at once.
A press that moves less than six pixels is still a click. Pressing a stepper and dragging further up
or down scrubs the number, a step every four pixels, shown in place: on release it goes into the open
field, or is committed once. A stepper keeps the usual cursor until it is dragged. Their tooltip names
the keys that do the same.

On a one-line element, or a group's header, the steppers and the chevron would cover the next
property, so where more of the line follows the value, its closing `/>` included, they take room in
the line, on hover and while a field is open, pushing the rest right until they go. In a sheet's
row, or at the end of a line, they are drawn over the gap and nothing moves, and so they are where
taking room would wrap a line that only just fits. An unset property's row moves its quiet `default`
aside. After a stepper click commits, they show again on the redrawn reading, so the next click steps
again, and a case's chevron stays after it closes its popover.

Only whole values edit: an attribute's value, an item of a braced list, or the value of a `let`
with no parameters. A click on a value edits it, as a form's field does, and leaves the selection as
it was, as a click on its steppers or chevron does: a field opens with the
caret where the click fell, a boolean toggles, and a case or a color opens its popover. A case shows a
chevron past it under the pointer, drawn and placed as a value's choices chevron is, room on a line
included; it stays as the pointer moves onto it, and a click on it opens the popover, or closes it. A click on
anything else, as a property's name, selects. Enter or F2 opens the selected unit's first value
with its text selected. Enter or leaving the field commits, Escape cancels, and Tab and Shift+Tab
commit and open the next or previous value, again leaving the selection as it was. An entry an editor cannot take, such as `12px` in a
number field, stays in the field, marked, and once typing pauses, or at once on Enter, a note beside
it says why. Fields take the value's own place and popovers open over the reading, so nothing else
moves. A string's field holds its value without quotes or escapes; a string with an escape other
than `\"` or `\\`, as `\n`, opens in the source instead. Where the reading shows a string without
quotes, as in a property sheet, as UI text or as an id, it reads `\"` and `\\` as the quote and
backslash they stand for; a string drawn as code, between its own quotes, reads as written.

On a selected row whose property has a default or is optional, Delete, or Cmd+Backspace (a Mac
laptop's delete key), fires `nx-edit` with `{ kind: "remove" }` for the attribute, and the element it
was on is selected. Backspace alone does not, since a reader who meant to change the text would
press it. A written row shows a quiet `⋯` at its end under the pointer and when selected; it opens a
menu, as do the context menu key and Shift+F10, offering Reset to default for a property with a
default (with the default, quieter, as `Reset to default  8`), Remove for an optional one, and Show in Source, which fires `nx-open` with the row's value.

### Moving and deleting items

An element that is an item of a list (a braced sequence, as `choices={ … }`, or what an element
holds, as a flow's steps) moves within that list and can be deleted, when the list holds no text and
the item is not inside an `if` branch or a `for` body. An `if` or a `for` that is itself an item stays
where it is unless moved past, and items it makes do not move.

- **Drag.** Where the reading draws the list an item to a line, an item of two or more is dragged by
  its icon and its name: the pointer shows a grab hand there, and the hover card ends with `Drag to
  reorder`. A press that moves 4 pixels starts the drag; a click still selects. The item is muted
  where it stands, a copy of one line follows the pointer, and a line in the selection's color marks
  the gap it would land in, above the comments directly over the item there; nothing else moves.
  Releasing on that line fires `nx-edit` with `{ kind: "move", to }`, the position the item takes;
  releasing elsewhere, beside the item, or Escape fires nothing. Near the top or bottom of what
  scrolls the reading, the drag scrolls it. A mouse, a pen and a touch all drag.
- **Keys.** With an item selected, Alt+Up and Alt+Down move it one place, in a list drawn on one line
  too; nothing happens at the ends. Delete, or Cmd+Backspace, deletes it with `{ kind: "remove" }`,
  without asking, since the host's undo takes it back, except the only item of a list whose type
  needs one or more (`Step+`).
- **Menu.** A movable or deletable item shows a quiet `⋯` at the end of its first line under the
  pointer and when selected; it opens a menu, as do the context menu key and Shift+F10, with Move up,
  Move down (each where the item has a neighbour that way), Delete and Show in Source, which fires
  `nx-open` with the item's key.

Items are keyed by their position (`steps[2]`), so a move changes keys. The element remembers the
moves and deletes it fired and reads the host's next tree with them: the moved item is selected,
or after a delete the item that took its place, else the one before it, else what held the list;
each fold on the list's items stays on its item, firing `nx-fold` with the new keys; and the live
region says `Moved to 3 of 8` or `Deleted`. A host sets `text` before `tree`, as it does after any
edit; a tree of the same text, as after a refusal, moves nothing. Until a tree comes, drags and the
other lists' moves and deletes wait, so a host sets the tree again after an edit it does not make
(refused, or answered with no edits). Keys pressed again before the next
tree, as Alt+Down held, fire edits keyed as the text will be once the ones before are made, so a host
that applies edits in order makes them all.

A selected element ends with a quiet row, `Show N unset properties` (`Hide unset properties` while
open), N being the properties it does not write; one
written under a property-list `if` or match counts as written, and an element of a union's case
offers its case's properties. A
click on it, or Enter, lists them as dimmed rows in its visualizer's order, else declaration order,
each with its default and a quiet `default`, or an optional one's type, quieter, in a place wide
enough to click and type in, and a filter field past twelve. Under a one-line element the rows start
where the element does. A dimmed row edits as a
written value does, opening on its default; a commit fires `nx-edit` with the element's key and
`{ kind: "addProperty", name, value }`, and the row reads as written until the host's tree. A
property no editor takes, such as a record, fires `nx-open` with the element's key on Enter or a
double-click. Escape, or selecting another unit, closes the list.

Enter or a double-click on a value the reading cannot edit, such as an expression or a reference to
a named value, fires `nx-open` with the node's `{ key, role, range }`, for the
host to show it in its editor. With the reading focused and no editor open, Ctrl or Cmd with Z or Y
are left to the host; inside a field, they are the field's. The element fires `nx-draft`, whose
`detail` is `{ changed }`, when an open editor comes to hold an entry that differs from its value and
when it no longer does, so a host can offer Undo for an edit not yet committed: pressing it moves
the focus, which commits the field, and the host then takes that edit back.

```ts
import { NX_EDIT_EVENT, NX_OPEN_EVENT, type NxEditDetail, type NxOpenDetail } from "@nx-lang/viewer";

viewer.editable = true;
viewer.addEventListener(NX_EDIT_EVENT, async (event) => {
  const { key, edit } = (event as CustomEvent<NxEditDetail>).detail;
  const text = model.getValue();
  const answer = await service.sourceEdit({ documents: [{ uri, source: text }], uri, key, edit });
  // Applied against the text the request carried; a refusal answers no edits.
  if (answer.edits.length > 0 && model.getValue() === text) {
    editor.pushUndoStop();
    for (const { range, newText } of answer.edits) {
      editor.executeEdits("visual", [{ range: toMonacoRange(range), text: newText }]);
    }
    editor.pushUndoStop();
  }
  // ...then set viewer.text and viewer.tree from the new text, as after any edit.
});
viewer.addEventListener(NX_OPEN_EVENT, (event) => showInEditor((event as CustomEvent<NxOpenDetail>).detail.range));
```

A host applies edits in the order they fire, each against the text the one before left.
`createEditQueue` does that for a host whose editor keeps the text, as Monaco does:

```ts
import { createEditQueue, NX_EDIT_EVENT, type NxEditDetail } from "@nx-lang/viewer";

const queue = createEditQueue(
  service, // anything with the language service's `sourceEdit`
  uri,
  {
    text: () => model.getValue(),
    // Every range is in the text as it is now; applied together, as one undo step.
    apply: (edits) => {
      editor.pushUndoStop();
      editor.executeEdits("visual", edits.map(({ range, newText }) => ({ range: toMonacoRange(range), text: newText })));
      editor.pushUndoStop();
    },
  },
  (text) => showTreeOf(text), // an edit changed the text: read it at once
  () => (viewer.tree = tree), // refused, unchanged or failed: give the element its tree again
);
viewer.addEventListener(NX_EDIT_EVENT, (event) => void queue((event as CustomEvent<NxEditDetail>).detail));
// Undo after the edits fired before it, as one pressed while a field's edit is still being asked.
const undo = () => queue.after(() => editor.trigger("visual", "undo", null));
```

An answer for text that changed while it was asked is not applied, and a failed edit does not hold
up the ones after it.

## Hover cards

Resting the pointer on a name or a value for half a second opens a card that says what it is, in the
editor's hover's style, from the source tree alone, so a host wires nothing. It is shorter than the
editor's where the reading shows the rest: a component reads as its name and base, not its tag with
every property, and a property reads unqualified, with its default:

| Under the pointer                                  | The card                                                 |
| -------------------------------------------------- | -------------------------------------------------------- |
| An element's name or icon                          | `component Shape extends Node` and the declaration's doc |
| A property's name, on a row or a line              | `(property) Type: ShapeType = Rectangle` and its doc     |
| An id or a title, shown without its property's name | That property's card                                    |
| A union case, bare or qualified                    | `(case) ShapeType.Circle` and the case's doc             |
| A reference, a member or a call                    | `size: int`, or the function's `let twice(n:int): int`, and the doc of what it names |

A literal, a comment, punctuation and a unit's room open nothing. The card stays while the pointer
moves onto it, so its text can be selected and its links followed, and closes once the pointer has
left both, when anything scrolls, on Escape, or when the text or the tree changes. It never moves the
selection, the folds or the scroll. With the reading focused, `i` shows the card of the selected unit
(an element's declaration, a row's property), and the live region reads it, or says "No information";
that card stays while the pointer moves, until Escape or the selection moves.

A doc's Markdown is rendered by a small built-in renderer that makes text and elements only:
paragraphs, inline and fenced code, emphasis, lists, and links, a link only to an `http:` or `https:`
address and opening in a new tab. Anything else, raw HTML included, reads as its text. A host that
renders Markdown its own way sets `markdown`, and answers for what it returns:

```ts
viewer.markdown = (doc) => {
  const holder = document.createElement("div");
  holder.innerHTML = sanitize(marked.parse(doc));
  return holder;
};
```

## Styling

The element follows the page's `color-scheme` through `light-dark()`. These custom properties
restyle it:

| Property                              | Default                                   |
| ------------------------------------- | ----------------------------------------- |
| `--nx-viewer-source-font-family`      | the system monospace stack                |
| `--nx-viewer-prose-font-family`       | the code's font, for comments and text shown to people |
| `--nx-viewer-indent-font-family`      | the system monospace stack, so indentation keeps its width |
| `--nx-viewer-source-font-size`        | `13px`                                    |
| `--nx-viewer-source-line-height`      | `18px`                                    |
| `--nx-viewer-foreground`              | `CanvasText`                              |
| `--nx-viewer-background`              | `transparent`                             |
| `--nx-viewer-selection`               | a light or dark blue tint                 |
| `--nx-viewer-selection-inactive`      | a gray tint, when the reading is not focused |
| `--nx-viewer-hover`                   | a faint tint                              |
| `--nx-viewer-group-gap`               | `0.25lh`, the room between groups         |
| `--nx-viewer-card-background`         | white, or a dark gray                     |
| `--nx-viewer-card-border`             | a light or dark gray                      |
| `--nx-viewer-link-color`              | a light or dark blue, for a card's links  |
| `--nx-viewer-edit-hover`              | a faint tint, on an editable value under the pointer |
| `--nx-viewer-field-background`        | white, or a dark gray, inside an open field |
| `--nx-viewer-field-border`            | a light or dark blue, the ring around an open field |
| `--nx-viewer-field-selection`         | a light or dark blue, selected text in a field |
| `--nx-viewer-invalid`                 | a light or dark red, the ring around an entry a field cannot take, and the border of the note saying why |
| `--nx-viewer-quiet-color`             | a gray, for a placeholder, a dimmed row's `default` and a choice's value |
| `--nx-viewer-more-color`              | a gray, for the unset-properties row      |
| `--nx-viewer-dimmed-opacity`          | `0.55`, for a property an element does not write |

## Development

```bash
pnpm install
pnpm --filter @nx-lang/viewer test
```

The tests run the element under jsdom with `node --test`, on source trees computed through
`@nx-lang/sdk-wasm` at test time from `test/fixtures/sources.json`. The coverage test renders every
`.nx` file in the repository and fails, naming the file and the key, for any node not rendered
exactly once.
