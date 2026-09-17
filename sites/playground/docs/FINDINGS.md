# What building this found

Every item here was found by trying to build the app, not by reading code. Each has a stable
number so it can be cited from a proposal, a commit, or another finding.

Only what is still open is listed. A finding that has been fixed is removed, and the numbers are
never reused, so a gap in the sequence is a finding that is gone: F1–F7, F13, F19–F22 and F27
so far. The git history of this file has their write-ups. What remains is worked around here, and
each is a candidate for its own change. The last group is not bugs at all — behavior that shaped
the catalog and is worth knowing before writing NX against it.

Unless a finding says otherwise, the Rust interpreter is the reference: where the two runtimes
disagree, the interpreter is the one that matches the language.

## Compiler and analysis gaps

### F8 — An unresolved type name is not reported by analysis

A type annotation naming a type that does not exist passes analysis and evaluates:

```nx
let x: NoSuchType? = null                        // evaluates to null, no diagnostic
external component <Box value: NoSuchType? />    // evaluates, no diagnostic
```

Code generation does reject it, with `type binding 'NoSuchType' is unavailable` — but only when the
whole program is built, and without pointing at the annotation (see F10).

### F9 — A visitor's declaration silently replaces the catalog's

The catalog is a module the visitor's source imports implicitly. A visitor who declares a component
the catalog also declares gets no diagnostic: their declaration shadows the catalog's, their own
properties work, and every catalog property on that tag is suddenly unknown. The diagnostic they
eventually see names a property rather than the collision:

```nx
external component <SkiaLabel mine:string? />
<SkiaLabel Text="x" />   // Element 'SkiaLabel' has no property 'Text'
```

Two declarations of one name inside a single module are reported; it is the collision across the
import that is silent.

### F10 — Whole-program code generation failures carry no position

`codegen-missing-semantic-data` (F8's diagnostic, among others) arrives with a zero-width span at
line 1, column 1 rather than a location. Marking that in an editor would assert the error is on the
author's first line, which is a lie, so the app classifies such diagnostics as `program` and reports
them without a position.

## Syntax gaps

### F11 — String escapes are scanned but not decoded

`"a\nb"` is six characters with a literal backslash; `\t` and `\"` behave the same way, though `\"`
does stop the literal from ending. String literals may span lines, and a newline in the source is a
newline in the value, so the examples write multi-line text that way and use single-quoted
attributes in embedded XML.

### F12 — A bracketed list literal is not accepted in a `for` header in content position

`for label in ["a", "b"] { ... }` inside an element fails with `Invalid element syntax`; binding the
list to a `let` first works. Both forms are fine outside content position.

## Found by wiring the readouts

Once `+` could join a number or a boolean to a string, the examples that had drawn a fixed
"Tapped 0×" or "SelectedIndex=1" were wired to the page's state, and the hand-numbered rows became
loops. Wiring them found four more. None blocks an example; each is worked around in the NX and
said so at the point of the workaround.

### F26 — Two events in one gesture: the first runs, the second is dropped

Choosing a radio button switches its neighbour off, and DrawnUI raises both `Toggled` events inside
the one tap. The renderer dispatches the first, which replaces the instance, and then drops the
second: its callback belongs to the drawing the dispatch just retired, and `DrawnTree` ignores a
callback whose instance has moved on. That guard is right for an event that arrives late, and wrong
for one that arrives in the same turn. The Common Controls page has each radio button report the
chosen option whichever event it receives, so either one produces the right line. A fix would queue
the events of one turn and run each against the instance the one before it left.

### F28 — An `if` without `else` is `void`, so a loop cannot filter and a handler cannot decline

The expressions reference shows `for n in numbers { if n % 2 == 0 { n } }` as a filter. The checker
types the `if` as `void` and the loop as `void[]`, which no content property accepts. The same rule
reaches handlers: `if action.value { <Card.Logged ... /> }` is rejected because a handler must
return an action, and a component with no state has no `<Update />` to return in the other branch.
The SkiaScroll page writes out one color list per row count instead of filtering a longer one.
The pending `type-conditionals-without-else` change addresses the sequence case.

### F29 — What the readouts still cannot say

Each of these left a note in an example where the original does more:

- **No range and no list indexing.** A loop needs a list of the right length, and a palette cannot
  be cycled by index, so SkiaScroll and Carousel & Drawer spell out their color lists.
- **No string length and no trim.** The Editor's password card cannot count its characters, and
  its chat card counts a message of only spaces, which the original skips.
- **No way to append to a list.** The Editor's chat card shows the latest message, not the last four.
- **No number formatting.** A `float64` has one text form, so a speed reads `1x` where the original
  pads it to `1.0x` with `toFixed(1)`.
- **No `else if`.** A three-way choice nests an `if` inside the `else` block.

## Engine integration

### F23 — A focused drawn editor kept every keystroke while Monaco had focus

DrawnUI's `SkiaEditor` subscribes to keys at window level and lets go only when another DOM text
field takes focus — its input proxy checks for an `INPUT`, a `TEXTAREA` or a contenteditable.
Monaco 0.56 edits through an EditContext host, a plain `div`, so after clicking a drawn editor in
the output pane and then into the source pane, typed characters went to the drawn editor and Monaco
saw only the navigation keys. Worked around in `src/editor/NxEditor.tsx`: Monaco taking focus
unfocuses `SkiaEditor.Focused`. The reverse direction needs nothing, since the proxy's hidden
textarea takes DOM focus from Monaco on its own. Worth reporting upstream.

### F24 — `RenderingMode="Default"` never takes effect, and twenty previews exhausted WebGL

The gallery asked for software rendering per preview precisely so that a page of canvases would
not exhaust the browser's WebGL contexts. It never got it: the engine creates its surface in the
`CanvasView` constructor, and the React wrapper assigns `RenderingMode` afterwards, so every
preview is a WebGL canvas. Twelve fit under Chromium's limit of about sixteen live contexts and the
gap went unnoticed; at twenty, the oldest canvases lost their context and drew blank, with a
shader-compilation error per frame. Pre-existing — the live site's twelve previews are WebGL too —
and worked around in `src/gallery/Gallery.tsx`: a card's canvas is mounted only while the card is
near the viewport, and unmounting disposes the view, which releases its context. Worth reporting
upstream.

### F25 — A drawn editor's typed text falls back to Skia's built-in face

`SkiaEditor` is a shape, not a label, so the demo's label style (`FontFamily: "FontText"`) never
reaches it. Its inner text and placeholder labels are built in code and pick the style up on
first measure, but every `UpdateLabel()` — on each keystroke — overwrites their `FontFamily` with
the editor's own, which defaults to `""`, and upstream now resolves `""` to Skia's built-in face.
The placeholder draws in OpenSans and the text typed over it draws in a monospace fallback. The
demo has the same configuration and the same result. Worked around in `editor.nx`, where every
editor names `FontFamily="FontText"` — the one place an example deviates from its original on
purpose. Worth reporting upstream.

## Not bugs — behavior worth knowing

### F14 — `Element[]?` does not accept an external component as content

A content property typed `Element[]?` rejects an external component child at runtime with
`expected Element, got Leaf`. Content in this catalog is therefore typed as a list of a declared
abstract external component. Worth noting because `docs/drawnui-proposal/ui/ui.nx` types its content
properties as `Element[]?`, which would not accept the components it is meant to hold.

### F15 — A wrapper's content property cannot be optional if it splices it

Declaring a wrapper's content `DrawnNode[]?` and splicing `{Children}` among literal siblings fails
with `expected DrawnNode[]?, found list object[]`; declaring it `DrawnNode[]` works. Catalog
components, which do not splice, are fine either way.

### F16 — A user-defined component must extend the catalog's node root to be content

Content is typed as a list of `DrawnNode`, and a plain `component <Card ... />` is not one, so every
wrapper in the examples is declared `component <Card extends DrawnNode ... />`. A consequence of how
the catalog is shaped rather than of NX, but it is the first thing anyone writing NX here hits.

### F17 — An unknown element name is not a compile error

`<NotAControl />` compiles: unknown tags are treated as intrinsic elements. A mistyped control name
therefore surfaces in the renderer as an unknown type rather than as a diagnostic, which is why the
renderer draws a visible placeholder and reports it instead of failing.

### F18 — Only abstract components may be extended

`external component <Leaf extends Middle />` where `Middle` is concrete is rejected with *only
abstract components may be extended*. Stated behavior with a clear diagnostic, not a gap — but it is
why the catalog emits an abstract twin (`SkiaLayoutBase`) for every control that is both a tag and a
base, and why a subclass may not restate an inherited prop. See `CATALOG.md`.
