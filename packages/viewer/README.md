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
| `folded`    | property               | The keys of what is folded; `undefined`, the default, folds the document's supporting declarations |
| `stale`     | property and attribute | The reading is muted and badged "Out of date"; selection still works         |
| `selection` | property               | The selected node's key; setting it scrolls the node into view               |

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
- **A property's element value** (`Padding=<Thickness …/>`) reads inline, as a line, unless it holds
  elements.
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
Lucide icon the element starts with; a kind inherits it), and a value's `format` of `id` or `color`.

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

A group folds to its header, followed by its title or a count of what it hides, and a declaration
written over several lines folds to its first line. A run of declarations that support the document
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
element on one line, or a block such as an `if`. Outside every unit, as in a function's body, it
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

## Development

```bash
pnpm install
pnpm --filter @nx-lang/viewer test
```

The tests run the element under jsdom with `node --test`, on source trees computed through
`@nx-lang/sdk-wasm` at test time from `test/fixtures/sources.json`. The coverage test renders every
`.nx` file in the repository and fails, naming the file and the key, for any node not rendered
exactly once.
