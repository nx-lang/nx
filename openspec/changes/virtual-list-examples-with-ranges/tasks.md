Paths without a repository prefix are in `~/src/DrawnUi.FiddleEngine`; `nx:` marks this repository.
Every fiddle command runs in side-by-side mode (`--nx ../nx`). Fiddle commits are local; nothing is
pushed, no pull request is opened, and no `@nx-lang/*` pin in `package.json` changes.

The fiddle's `nx-linkable-ir` branch already carries the previous change's uncommitted repin to the
published `0.2.0`, which is where this change wants the pins, and it touches three files this change
also edits. By the user's decision the work here is built on top of it and committed together, so a
fiddle commit below carries those hunks as well.

## 1. NX: the spec text and the diagnostic

- [x] 1.1 Correct `nx:openspec/specs/range-expressions/spec.md` per the delta: every scenario that
  writes a range as an unbraced binding initializer takes the braced form, and the new scenario
  pinning `let r = 1..5` as a parse error is added; verify each corrected snippet by running it
  through `cargo run -p nx-cli -- run` and seeing the behavior the scenario states
- [x] 1.2 Add the parser or checker test that `let r = 1..5` is rejected the way `let n = 1 + 2` is,
  alongside the existing range tests in `nx:crates/nx-syntax/tests/parser_tests.rs`; verify it fails
  before the assertion is written correctly and passes after
- [x] 1.3 Fix the bare-name diagnostic in `nx:crates/nx-types` so an unresolvable bare name at a
  non-union, non-type-parameter site suggests `{name}` when a value of that name is lexically
  visible, and offers the quoted form only where a string satisfies the site; verify with new tests
  that `let r = 5` + `let x:int = r` names `{r}` and never `"r"`, and that a `string` site still
  offers the quoted form
- [x] 1.4 Run `cargo test` and `pnpm -r test` in `nx:` and verify both are green before touching any
  example

## 2. Playground: the list examples count with a range

- [x] 2.1 Rewrite `nx:sites/playground/src/examples/nx/cells.nx` so `contacts` is one `for` over
  `1..=100000` calling a per-contact function, deleting `digits`, `Contacts`, `Shift`, `Tenfold` and
  `seed` and the "NX has no range" comment; verify `pnpm run check-examples` passes and the file no
  longer mentions a range as missing
- [x] 2.2 Rewrite `uneven-cells.nx` the same way over `1..=200`, deleting `digits`, `Posts`,
  `Shift`, `seed`, `hundred` and the comment; verify `pnpm run check-examples` passes
- [x] 2.3 Rewrite `scroll.nx`: each numbered row list becomes a `for` over a range, the palette
  becomes `color(index:int)` cycling the original's eight colours with `%`, and `colors10`,
  `colors12`, `colors14` and the rest go; move the note about list indexing to the colour function
  and verify the rows still show the same colours in the same order as before the rewrite
- [x] 2.4 Re-read the three files' header comments against what NX now does and verify no source in
  `src/examples/nx/` says NX has no range (`grep -rn "no range" src/examples/nx/` finds nothing)
- [x] 2.5 Verify the coverage metadata in `src/examples/examples.json` is unchanged for all three —
  each was `static` with `code-behind` before and stays so, since what they lack is a way to reach
  the control from code, not a way to count — and that `pnpm test` passes in `nx:sites/playground`
- [x] 2.6 Run the playground (`pnpm dev`), open Cells, Uneven Cells and SkiaScroll in a browser, and
  verify each draws and scrolls as it did, recording compile-plus-evaluate time and compiled IR size
  before and after for Cells in the commit message — the playground has no share link, so the IR is
  what its size is measured on
- [x] 2.7 Commit the playground rewrite

## 3. Fiddle: the catalog declares templated controls

- [x] 3.1 Port the templated-control rule from `nx:sites/playground/scripts/generate-catalog.mjs`
  into `nx/generate-catalog.mjs`: a class declaring the collection and the cell factory gets
  `TItem:type`, `ItemsSource: TItem[]?` and `ItemTemplate: (<function Item:TItem Index:int />:
  DrawnNode)?`; verify with a generator test that `SkiaLayoutBase` declares all three and that a
  control below inherits them
- [x] 3.2 Record the template's parameter names and the collection property per renderable control
  in `catalog-meta.json`, extend the metadata types in `nx/src/values.ts`, and verify `tsc` passes
  through `npm run test:nx -- --nx ../nx`
- [x] 3.3 Regenerate with `npm run catalog -- --nx ../nx`, review the diff of `drawnui.nx`,
  `catalog-meta.json` and `omitted.json` member by member for anything the change moved besides the
  three properties, and verify `omitted.json` no longer lists `ItemTemplate` while non-event
  functions such as `ProcessJson` remain
- [x] 3.4 Update the catalog test in `nx/test/catalog.test.ts` that asserts `ItemTemplate` is
  omitted, replacing it with one asserting the templated shape is declared; verify
  `npm run test:nx -- --nx ../nx` passes
- [x] 3.5 Update `nx/CATALOG.md` so "How it is derived" describes templated controls and no longer
  says functions that are not events have no NX expression; commit

## 4. Fiddle: the renderer draws cells

- [x] 4.1 Port `nx:sites/playground/src/render/templateCell.ts` to `nx/src/templateCell.ts` with the
  origin noted in the header; verify `tsc` passes. Two things could not be copied, and the header
  says so: the playground draws by materializing controls and so already had a materializer, while
  the fiddle draws through React and a cell is not a render, so `materialize.ts` came over as
  `buildControls`; and the package the fiddle draws with ships `SkiaDynamicDrawnCell` for exactly
  this, so the cell extends that and overrides `SetContent` rather than subclassing `SkiaLayout`
- [x] 4.2 Bind a function record on a template property to DrawnUI's cell factory in `nx/src/values.ts`
  and `nx/src/draw.tsx`, passing the recorded parameter names, and make the binder part of the
  cell's own context so a templated control inside a cell still templates; verify with a draw test
  that a nested templated control renders its cells
- [x] 4.3 Report a failing template call in the host's console as `{template} at index {n}: {message}`
  without taking down the drawing; verify with a test that one failing item leaves the other cells
  standing and produces exactly one console line
- [x] 4.4 Add runtime tests: a snippet binding `TItem`, `ItemsSource` and `ItemTemplate` compiles and
  draws its cells, the template is called with the item and its index, and a template whose `Item`
  parameter is the wrong record type fails with an `L{line}:` error; verify they pass with
  `npm run test:nx -- --nx ../nx`
- [x] 4.5 Commit

## 5. Fiddle: the virtual-list presets

- [x] 5.1 Add the **Cells** preset to `NxAll` in `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`, ported from
  the playground's rewritten `cells.nx` with the fiddle's fonts and host-served assets, its
  collection one `for` over `1..=100000`, `RecyclingTemplate=Enabled` and
  `MeasureItemsStrategy=MeasureFirst`; verify it compiles in the runtime tests and its share stays
  under 128 KB
- [x] 5.2 Add the **Uneven Cells** preset the same way from `uneven-cells.nx`, over `1..=200` with
  `MeasureItemsStrategy=MeasureVisible`; verify the same two things
- [x] 5.3 Write each preset's description in `NxAll` to say what it shows and that it is
  non-interactive, and verify neither description nor source claims NX cannot express a collection,
  a template or a count
- [x] 5.4 Extend `nx/test/runtime.test.ts` so the preset sweep covers the two new entries — they
  bind no handler, so assert they draw and that the dispatch sweep still finds none — and verify the
  whole preset suite passes with `npm run test:nx -- --nx ../nx`
- [x] 5.5 Run `npm run runtime -- --nx ../nx`, start the dev host and the backend, open both presets
  in the fiddle, and verify each draws and scrolls a hundred thousand and two hundred rows
  respectively
- [x] 5.6 Commit

## 6. NX fixes the port turned up

- [x] 6.1 Keep a running note of every compiler bug met while porting, with the smallest source that
  reproduces it; verify each entry reproduces from a clean `cargo run -p nx-cli -- run`

  Two, both found before the porting rather than during it, and both fixed in group 1:

  1. **The range spec wrote a form the language rejects.** `let r = 1..5` → `Syntax error`, as
     `let n = 1 + 2` is, because `unbraced-literal-forms` makes an unbraced value a literal.
     Scenario text only; the implementation and its tests were already right.
  2. **The bare-name diagnostic proposed a form the site rejects.** `let r = 5` + `let x:int = r`
     answered "for a string value write \"r\"" at an `int` site and never named `{r}`.

  The port itself turned up none. The three playground rewrites and both fiddle presets compiled
  and drew without a compiler change, which is what the design predicted: ranges were already
  working end to end on this branch, and what the fiddle lacked was catalog and renderer support,
  not language support.

  Two defects were found and fixed outside NX, recorded here so they are not mistaken for
  compiler bugs: the fiddle's `declarationOf` test helper stopped at the first `/>`, which is now
  inside `ItemTemplate`'s own function type, and its preset sweep counted any function-valued prop
  as a bound handler, which a cell factory is not.
- [x] 6.2 Fix each one whose fix is contained to one crate and its tests, with a regression test per
  fix, and verify `cargo test` and `pnpm -r test` stay green — both were; both are
- [x] 6.3 For anything larger, write a one-paragraph problem statement naming the reproducer and add
  it here as a deferred item rather than fixing it; verify each deferred item names the capability it
  belongs to — nothing was deferred
- [x] 6.4 If any fix changed compiler behavior, rerun `pnpm run check-examples` in
  `nx:sites/playground` and `npm run test:nx -- --nx ../nx` in the fiddle, and verify both are green

## 7. Close out

- [x] 7.1 Verify the spec deltas match what was built: read `specs/playground/spec.md`,
  `specs/fiddle-nx-language/spec.md`, `specs/range-expressions/spec.md` and
  `specs/unbraced-literal-forms/spec.md` against the code and correct either side where they disagree
- [x] 7.2 Run `openspec validate --strict virtual-list-examples-with-ranges` and verify it passes
- [x] 7.3 Verify the fiddle's `package.json` pins still read `0.2.0` — moved there by the prior work,
  not by this change — and that its branch is unpushed
  (`git status`, `git log origin/..HEAD`), and state in the final summary what the next release must
  carry for the presets to work from npm
