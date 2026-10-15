# @nx-lang/viewer

An `<nx-viewer>` custom element that shows NX source as a reading: the document's own text, with each
element read the way a design tool's layers read rather than as tags. An element written over several
lines reads as a group under its name, with a row to each property and no angle brackets or closing
tag; an element on one line reads as its name and properties; values lose their quotes; and the rest
reads as written, in the colors of the host's editor. It has no framework dependency and keeps its
markup and styles in a shadow root, so it drops into a plain page, a React app, a docs site or a VS
Code webview alike.

What it shows is the source tree the NX language service answers for a document: `sourceTree(uri)`
on a language snapshot of [`@nx-lang/sdk-wasm`](../../bindings/wasm/README.md), or the same query
through any `NxLanguageService`. The reading keeps every node: each node of the tree is rendered once
and carries its key, so a host can move between the reading and its editor.

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
        Partial<Pick<NxViewerElement, "tree" | "text" | "colors" | "icons" | "stale" | "selection">>;
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
| `stale`     | property and attribute | The reading is muted and badged "Out of date"; selection still works         |
| `selection` | property               | The selected node's key; setting it scrolls the node into view               |

## How it reads

- **An element over several lines** reads as a group: a header with its icon, its name in bold, its
  id, and the attributes written on its first line as `name=value` (with `idPosition=end`, the id
  follows those attributes); then a row for each attribute
  written on a line of its own, names in one quiet column and values in another; then what it holds,
  indented. There are no angle brackets and no closing tag. A bracket runs down the group from under
  its name and turns right at its end. A value of several lines reads under its name, and so does
  every value in a pane narrower than about 64 characters.
- **An element on one line** reads as its icon and name, its title's value alone, each other attribute
  as `name=value`, and what it holds as written. When it shares its line, with a property's name or a
  `let`, it keeps a quiet `<` and `/>` to show where it starts and ends. Alone on its line, in a place
  typed exactly as its kind (a `Choice` in `choices: Choice+`), it leaves out its name, which its
  icon carries.
- **What a group holds** reads an item to a line. An `if` or a `for` block reads as written, its
  later lines indented from its own start. A blank line between two items, and the space between two
  siblings where either is a group, becomes a little room.
- **Values**: strings lose their quotes, and booleans read ✓ and ✗. A string typed `uitext` (an alias
  `type uitext = string`, or any alias that leads to it) reads as text shown to people, in the prose
  font. A string a visualizer names as an element's `id`, or formats as `id`, reads as one quiet token; a
  string formatted as `color` reads with a swatch.
- **Everything else** (declarations, function bodies, comments) reads as written, in `colors`.
- **Long lines** wrap a step in from where their unit starts: a group's header, a row's value, an
  element on one line alone on its line, and an item a group holds as written. Code read as written
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
