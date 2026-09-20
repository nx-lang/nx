## Why

The two virtualized-list examples in the playground — Cells and Uneven Cells — open on a comment
saying "NX has no range", followed by twenty lines of scaffolding that exist only to count:
a seed of ten items, an element function that shifts them by an offset, and four nested calls that
multiply ten into a hundred thousand. NX has a range now (`0..n`, `1..=n`, and `for` over either),
so the scaffolding is obsolete and the comment is false. An example that says the language cannot
do something it can do is worse than no example: the playground is where a visitor forms their
first opinion of NX, and the list examples are the ones that show it holding real data.

The same lists cannot be written in the DrawnUI fiddle at all. Its catalog generator omits
`ItemTemplate` as "a function that is not an event", written when NX had no function types, so the
fiddle's `SkiaLayout` has no `TItem`, no `ItemsSource` and no `ItemTemplate` and no preset can bind
a collection. Function types landed in 0.2.0 and the playground's generator already emits the
templated shape; the fiddle's has not caught up.

## What Changes

Paths without a repository prefix are in `~/src/DrawnUi.FiddleEngine`; `nx:` marks this repository.

### The playground's range-blocked examples

- `nx:sites/playground/src/examples/nx/cells.nx` builds its hundred thousand contacts from
  `for id in 1..=100000`, dropping `digits`, `Contacts`, `Shift`, `Tenfold` and `seed`.
- `uneven-cells.nx` builds its two hundred posts from `for id in 1..=200`, dropping the same
  scaffolding.
- `scroll.nx` drops `colors10`, `colors12`, `colors14` and the rest of the written-out palettes:
  each row list becomes a `for` over a range, and the colour comes from a function that cycles the
  eight-colour palette with `if index % 8 is`. NX still has no list indexing, and the source says so
  where the palette is chosen rather than where the collection is built.
- Every comment claiming NX has no range goes with the code it excused. All three stay `static`
  with `code-behind`, their coverage tags unchanged: what they lack is a way to reach the control
  from code, and neither ranges nor list indexing are in the capability vocabulary.

### The fiddle's catalog and renderer

- `nx/generate-catalog.mjs` learns the templated-control shape the playground's generator already
  produces: a class declaring `ItemsSource` and `ItemTemplate` gets a `TItem:type` parameter,
  `ItemsSource: TItem[]?` and `ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`, with
  the template's parameter names recorded in `catalog-meta.json`. `ItemTemplate` leaves
  `omitted.json`; other non-event functions stay.
- The fiddle's renderer draws a templated control: a function record bound to a template property
  becomes DrawnUI's cell factory, and a cell calls the template with the item and index DrawnUI
  binds to it — the `templateCell.ts` and `materialize.ts` mechanism from
  `nx:sites/playground/src/render/`, ported as the instance tree and the handler binding were.
- **BREAKING** for a stale catalog: `nx/catalog/drawnui.nx`, `catalog-meta.json`, `omitted.json`
  and the committed catalog image are regenerated, so a checkout that does not re-emit the image
  fails the runtime build's staleness check, as it is meant to.

### The fiddle's presets

- Two virtual-list presets join `NxAll`, ported from the playground's rewritten examples with the
  fiddle's own fonts and host-served assets: **Cells**, a hundred thousand recycled cells with
  `RecyclingTemplate=Enabled` and `MeasureItemsStrategy=MeasureFirst`, and **Uneven Cells**, a feed
  of rows whose heights follow their text with `MeasureItemsStrategy=MeasureVisible`.
- Both are built from ranges, so each preset's source is the list it draws and nothing else.

### NX itself

- `range-expressions`' scenarios are corrected where they write a range as an unbraced binding
  initializer (`let r = 1..5`). That form does not parse and should not: `unbraced-literal-forms`
  requires an unbraced value to be a literal, and `let n = 1 + 2` is rejected for the same reason.
  The implementation and its tests use `let r = {1..5}`; only the spec text is wrong, and it is the
  text an example author reads.
- The diagnostic for a bare name that does not resolve stops proposing a form the site would
  reject. At an `int` site, `let x:int = r` today answers "a bare name resolves only against a
  union's cases; for a string value write \"r\"" — a string is not accepted there, and the fix the
  author wants, `{r}`, is never mentioned. Where a value of that name is lexically visible the
  diagnostic names the braced form; the quoted form is offered only where a string fits. This is
  the first message an author meets after writing `let span = 1..=5`, which is how it was found.
- Any further compiler bug the port turns up is fixed here when the fix is contained to one crate
  and its tests; anything needing a language-design decision is written up as its own proposal
  instead, named in this change's tasks.

### Not in scope

- The fiddle's `@nx-lang/*` pins stay at the published `0.2.0`, which predates ranges. Everything
  fiddle-side is built and tested in side-by-side mode (`npm run runtime -- --nx ../nx`,
  `npm run test:nx -- --nx ../nx`), as the previous fiddle change did. Until the next NX release is
  tagged and the pins move, the fiddle's npm-pinned build cannot compile the new presets; moving
  the pins and opening the pull request follow the release and are not part of this change.
- List indexing, a range with a step, and the jump toolbars and `LoadMore` paging the DrawnUI
  originals drive from code-behind.

## Capabilities

### New Capabilities

None. Every behavior this change touches is already described by an existing capability.

### Modified Capabilities

- `playground`: the two list examples are required to build their collections from a range rather
  than by hand, and no example's source may claim NX has no range.
- `fiddle-nx-language`: the fiddle's generated catalog declares templated controls and stops
  omitting `ItemTemplate`; the renderer draws a templated control's cells; the presets include a
  virtualized list.
- `range-expressions`: scenarios that write a range as an unbraced binding initializer are
  corrected to the braced form the language accepts.
- `unbraced-literal-forms`: the diagnostic for an unresolvable bare name must offer a form the site
  accepts — the braced reference where a binding of that name is visible, the quoted form only
  where a string fits.

## Impact

- `nx:sites/playground/src/examples/nx/{cells,uneven-cells,scroll}.nx`, checked by
  `pnpm run check-examples`.
- `nx:openspec/specs/range-expressions/spec.md` scenario text, and the bare-name diagnostic in
  `nx:crates/nx-types` with its tests.
- In the fiddle: `nx/generate-catalog.mjs`, `nx/catalog/drawnui.nx`, `nx/catalog/catalog-meta.json`,
  `nx/catalog/omitted.json`, the committed catalog image, `nx/src/values.ts`, `nx/src/draw.tsx`,
  new `nx/src/templateCell.ts`, `nx/CATALOG.md`, `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`, and the
  tests under `nx/test/`.
- Nothing ships to npm or NuGet, and no release is cut.
