## Context

See proposal.md — Why. What shapes the approach is where each piece gets its compiler and how far
apart the two example sets have drifted.

- **The playground builds from this checkout.** `sites/playground` depends on `@nx-lang/*` as
  `workspace:*`, so a `pnpm -r build` plus a wasm SDK build is all its examples need to use ranges.
  Verified against this branch: the interpreter evaluates `{ for i in 1..=100 { … } }`, the
  TypeScript emitter lowers a range `for` to `nxRangeMap` over a `Range` record without
  materializing the integers, and the IR runtime reads the same record. Nothing in the language is
  missing for the rewrite.
- **The fiddle builds from pinned packages.** `~/src/DrawnUi.FiddleEngine` pins `@nx-lang/*` at the
  published `0.2.0`, which predates ranges, and takes a checkout instead only when run with
  `--nx ../nx`. The previous fiddle change (`2026-09-18-update-fiddle-to-latest-nx`) established
  that convention, including leaving the pins alone and keeping the commits local.
- **The two catalog generators have diverged.** `sites/playground/scripts/generate-catalog.mjs`
  knows the templated-control shape — `TEMPLATE_ITEMS`, `TEMPLATE_FACTORY`,
  `TEMPLATE_TYPE_PARAMETER` — and emits `TItem`, `ItemsSource` and `ItemTemplate` on
  `SkiaLayoutBase`. The fiddle's `nx/generate-catalog.mjs` does not; `nx/CATALOG.md` still says
  functions that are not events "have no NX expression yet", written before function types landed,
  and `nx/catalog/omitted.json` lists `ItemTemplate` — a fact one of its own tests asserts.
- **Only the playground has a cell renderer.** `sites/playground/src/render/templateCell.ts` turns a
  function record into DrawnUI's cell factory, and `materialize.ts` and `DrawnTree.tsx` bind it. The
  fiddle's `nx/src/draw.tsx` has no equivalent, because nothing in its catalog could carry one.

## Goals / Non-Goals

**Goals:**

- One way to count in NX examples, in both repositories: a `for` over a range.
- The fiddle able to express a virtualized list at all — catalog, renderer and preset.
- Every claim an example's source makes about the language true on the day it is read.

**Non-Goals:**

- Publishing `@nx-lang/*` or moving the fiddle's pins. Both follow the next release.
- Giving NX list indexing, a stepped range, or a way to call into a control. `scroll.nx` still
  cycles its palette with `%`, and the originals' jump toolbars and `LoadMore` paging stay out.
- Re-porting examples that are not blocked by the absence of a range.

## Decisions

### The collection stays a list; the range only builds it

`ItemsSource` is typed `TItem[]`, so a range cannot be bound to it directly. The examples keep
producing a list and only change how it is produced:

```nx
let contacts = { for id in 1..=100000 { contact(id) } }
```

`contact(id)` is the existing per-item function. The alternative — widening `ItemsSource` in the
catalog to accept a range, so DrawnUI could realize items lazily from bounds — is a catalog and
renderer design of its own, and DrawnUI's own API takes a collection. It is not attempted here.

This materializes a hundred thousand records at evaluation, which is what the nested-`Tenfold`
version already did; the loop replaces four levels of element-function calls with one, so the work
should go down, not up. The tasks measure it rather than assume it, and the IR gets smaller because
five declarations become one.

### `scroll.nx` gets a colour function, not a palette per length

The written-out `colors10`, `colors12`, `colors14` lists exist only because a `for` needed something
to iterate and nothing could index the palette. A range gives the first half; the second stays
missing, so the palette becomes a function over the index:

```nx
let color(index:int): string = { if index % 8 is { 0 => "#0F3460" … } }
```

The source says NX has no list indexing at that function, which is true, rather than at the
collection, which is no longer true. The alternative — keeping one palette list and iterating it —
cannot produce the twelve and fourteen-row cards the original shows.

### The fiddle's generator learns the rule; the catalog is not hand-edited

`nx/catalog/drawnui.nx` is generated, and `nx/CATALOG.md` states that overrides are for what the
package's types cannot say. A hand-written override for `TItem`/`ItemsSource`/`ItemTemplate` would
be dropped at the next `drawnui-react` resync — exactly the trap the event work avoided by porting
detection into the generator. So the templated-control rule is ported from the playground's
generator, the way event detection was, and `ItemTemplate` leaves `omitted.json` as the event
callbacks did.

### The cell renderer is copied from the playground, with its origin in the header

`templateCell.ts` is ~80 lines whose difficulty is entirely in DrawnUI's binding protocol: the cell
is a host control whose content is recomputed whenever DrawnUI binds it to an item. Writing a second
one against the same protocol would be a second chance to get that wrong. The previous fiddle change
copied `instances.ts` the same way and noted its origin in the file header; this follows it. The
duplication is a known cost, recorded below.

### The unbraced range is a spec fix, not a language change

`let r = 1..5` does not parse, and `unbraced-literal-forms` says why: an unbraced value is a literal
and never an expression — `let n = 1 + 2` is rejected identically. The range spec's scenarios were
written in the shorthand, and the implementation's own tests all brace. Making the language accept
unbraced binary expressions would overturn an invariant several other capabilities rest on, for the
sake of a spelling. The scenarios are corrected instead, and one is added that pins the rejection so
the shorthand does not come back.

### The bare-name diagnostic names the braced form

`let x:int = r` answers with a suggestion to write `"r"`, which an `int` site would also reject, and
never mentions `{r}`. `unbraced-literal-forms` already requires the diagnostic to "indicate the form
the site does accept"; the fix is to consult what is lexically visible and the expected type before
choosing the suggestion. Contained to the diagnostic's construction and its tests.

### Side-by-side only, and the fiddle's commits stay local

Chosen by the user over cutting a `0.3.0`. Everything fiddle-side runs as
`npm run runtime -- --nx ../nx`, `npm run test:nx -- --nx ../nx` and `npm run catalog -- --nx ../nx`.
No pin moves, nothing is pushed, and no pull request is opened — the same posture as the last fiddle
change.

## Risks / Trade-offs

- **The fiddle's npm-pinned build cannot compile the new presets until `0.3.0` ships.** → The work
  stays on a local branch, unpushed, with no pin change and no PR. The failure a maintainer would
  see if they built against the pins is a compile error on the range, which names the line. The
  release and the repin are named as the follow-up in the proposal's "Not in scope".
- **A hundred thousand records evaluated in the browser.** → Already true today; the tasks measure
  compile-plus-evaluate time and share size before and after, and the numbers go in the commit
  message. If the range version is slower, that is a finding worth a bug, not a reason to keep the
  scaffolding.
- **`templateCell.ts` now exists twice.** → Copied verbatim with the origin in the header, so a
  future fix can be applied to both by searching for the header. Extracting a shared package is the
  right answer once the fiddle takes `@nx-lang/*` from npm again, and is out of scope here.
- **Regenerating the fiddle's catalog moves the committed image.** → The runtime build already fails
  on a stale image and names the regeneration command, so a checkout that misses the step is told.
  The generated diff is reviewed member by member in its own task, since a generator change can
  silently move unrelated members.
- **The bug budget can be spent in one place.** → A compiler fix that outgrows one crate and its
  tests is written up as its own proposal and named in the tasks rather than absorbed, so this
  change stays reviewable.

## Migration Plan

Nothing deploys. The order is: NX first, then the playground, then the fiddle.

1. Fix the range spec's scenarios and the bare-name diagnostic in this repository; `cargo test` and
   `pnpm -r test` green.
2. Rewrite the three playground examples; `pnpm run check-examples` green, the gallery inspected in
   a browser for the two list examples and `scroll`.
3. In the fiddle, against this checkout: generator, catalog regeneration, renderer, presets, each
   its own commit, `npm run test:nx -- --nx ../nx` green after each.

Rollback is per commit; the catalog image is reproducible from `npm run catalog -- --nx ../nx`.
