## Why

The DrawnUI fiddle (`~/src/DrawnUi.FiddleEngine`) runs NX 0.3.0. Since that release, this repository
has made two breaking language changes, `flat-sequences` and `occurrence-cardinality`. They remove
`T[]`, `null` and the ternary, move optionality onto a property's name (`x?: T`), and raise the IR to
schema 5. None of that has reached the fiddle yet. Its catalog generator still writes `Text: string?`
and `Children: DrawnNode[]?`. Its presets use `DrawnNode[]`, `string[]` and a ternary. The
`drawnui.nxir` it bundles is a schema-4 image. So the fiddle stops building the moment it is pointed
at this checkout.

Meanwhile the fiddle has moved on as well. Its maintainers took `drawnui-react` from 0.1.0-preview.9
to preview.12, the latest on npm, and regenerated the catalog under NX 0.3.0 (`IsParentIndependent`,
the new `SkiaScrollBar` members, one more omitted member). That work sits on the fiddle's `main`, so
this change starts there.

The NX playground in this repository (`sites/playground`) lags further. It draws with a copy of
DrawnUi.React's source vendored on 2026-09-08 at 0.1.0-preview.4, because upstream was then a
private, unbuilt package. That is no longer true: `drawnui-react` is published on npm, and the
fiddle already draws with it. The playground misses eight releases of DrawnUI fixes and controls,
its catalog is derived differently from the fiddle's (from the reconciler's registry, not the
package's React entry), and it carries 776 KB of vendored source that nothing edits.

The fiddle is the public DrawnUI playground for NX. Running it against this checkout is also the last
end-to-end check of the two language changes before they are published.

## What Changes

- **Start from the fiddle's current `main`.** A new fiddle branch starts from `origin/main`, which
  carries preview.12 and its regenerated catalog. If npm has published a `drawnui-react` newer than
  the pin by the time the work starts, the pin moves and the catalog is regenerated in the same step.
  Either way the catalog's `CATALOG.md` stops naming preview.9 as the version it is generated from.
- **Side by side with this checkout.** The fiddle is built and tested against this checkout
  (`npm run runtime -- --nx ../nx`, `npm run test:nx -- --nx ../nx`) after `pnpm install &&
  pnpm -r build` here.
- **The catalog generator writes the new spelling.** The generator follows the rules the NX
  playground's generator already follows, and the `fiddle-nx-language` spec already states:
  - `x?: T` for every author-settable property and every record field;
  - `x?: T+` for a DrawnUI array;
  - `TItem+` for `ItemsSource`;
  - `ItemTemplate?: <function Item:TItem Index:int />: DrawnNode`, with no parentheses;
  - `content Children?: DrawnNode+`.

  `drawnui.nx` and its header comment (which still talks about the renderer dropping nulls) are
  regenerated, and `drawnui.nxir` is re-emitted as a schema-5 image.
- **The presets, tests and docs are rewritten in the new spelling:**
  - the six NX presets in `FiddlePresetsNx.cs`: `content Children:DrawnNode+`, `colors:string+`, and
    `if … else` in place of the ternary;
  - the NX sources embedded in `nx/test/*.test.ts`;
  - the examples in `README-NX.md`, which `readme.test.ts` compiles;
  - `nx/CATALOG.md`.
- **The renderer's absence handling is checked against the new wire form.** In the new wire form an
  absent optional field is omitted, and `{}` is the one empty value. `values.ts` already drops
  `null` and `undefined` and treats a missing content property as no children. This change confirms
  that holds and adds tests where an `if` with no `else` or an unwritten optional property reaches a
  control. Names left over from `null` in the fiddle's renderer are renamed.
- **A share from before the upgrade is reported, not broken.** A schema-4 share stored under 0.3.0
  cannot be read by a schema-5 runtime. Instead, the `/p/` player draws the engine's failure label
  naming both schema versions. The share's editor link opens its source unchanged. There, the
  compiler's fix-its (`T[]` → `T*`/`T+`, the `name?:` form) show the author how to migrate it.
- **NX issues the fiddle turns up are fixed here or recorded.** An issue with a modest fix is fixed
  in this checkout, with a test, before the release. One too large for now is recorded in
  `specs/future.md` and worked around in the fiddle, with the workaround pointing at the entry.
- **The NX playground draws with the published `drawnui-react`.** The playground:
  - depends on `drawnui-react` pinned exactly to the fiddle's version (preview.12 today);
  - deletes the vendored `src/drawnui/` tree and imports from the package's entries (`drawnui-react`
    and `drawnui-react/core`);
  - rewrites `render/materialize.ts` to build controls from the classes the package exports, as the
    fiddle's cell builder does, since `Registry` and `applyProps` are internal to the package;
  - generates its catalog from the package's declarations by porting the fiddle's generator, so the
    two catalogs are derived the same way and differ only where each records why. `catalog-meta.json`
    records the package version, and the playground's tests fail when it disagrees with the pin.

  The fonts, images, Lottie files, sprite sheets, shaders and reference demo pages are not in the
  package. So `sync-drawnui` still copies them, now from the upstream tag matching the pin
  (`v0.1.0-preview.12`), read with `git archive` so the upstream working tree is never touched. It
  records the tag it copied. Every gallery example still compiles (`check-examples`) and draws in a
  browser. An example that the new DrawnUI breaks is fixed. A demo page upstream added since
  preview.4 is not ported. Instead, the site's documentation names it and says why, as the playground
  spec requires for any demo page without an NX example. The playground's README, `docs/CATALOG.md` and Dockerfile follow.
- **Release and repin, each on the user's go-ahead.** Once everything passes against the checkout:
  1. The NX branch, playground included, reaches `main`.
  2. `v0.4.0` is tagged and published per `docs/deployment.md`. The bump is minor under 0.x because
     the changes are breaking.
  3. The fiddle's `@nx-lang/*` pins move to 0.4.0.
  4. Its suites run without `--nx`.
  5. The fiddle branch is pushed and its pull request opened.

Out of scope:
- the engine's C# and TSX languages;
- new presets or interaction;
- porting further playground examples, or DrawnUI demo pages new since preview.4;
- keeping schema-4 shares playable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `fiddle-nx-language`: a share whose NX IR schema differs from the runtime's is reported by the
  player as a failure label naming both schema versions, and its editor link still opens its source.
  The catalog-spelling requirements already state the occurrence forms (`occurrence-cardinality`
  updated them), so they do not change.
- `drawnui-nx-catalog`: the playground's catalog is derived from the pinned `drawnui-react`
  package's declarations rather than vendored sources, records that version, and is detected as
  stale when the pin moves. Divergence records no longer mention edits to a vendored copy.
- `optional-properties` (found by the fiddle run): a property rejected for `name: T?` is read as
  marked, so leaving it out at a use is not reported a second time.
- `editor-language-service` (found by the fiddle run): a property completion's detail is
  `name?: T`, as hover shows it. It was the type alone, which lost the optional mark once the mark
  moved onto the name.
- `playground`: the site draws with the pinned `drawnui-react` package. Its documentation describes
  moving the pin and refreshing the assets copied from the matching upstream tag, whose provenance
  is recorded.

## Impact

- `~/src/DrawnUi.FiddleEngine`, on a new branch from `origin/main`:
  - `nx/generate-catalog.mjs` and `nx/catalog/*`: the new spelling, regenerated, with a schema-5
    image;
  - `nx/CATALOG.md`, `README-NX.md` and `AGENTS.md`;
  - `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`;
  - `nx/src/values.ts`, `nx/src/runtime.ts` and whatever the schema-5 run turns up;
  - `nx/test/*`;
  - `package.json` and `package-lock.json`, after the release only.
- This repository:
  - `openspec/specs/fiddle-nx-language`, `drawnui-nx-catalog` and `playground`;
  - `sites/playground`: `package.json` and `pnpm-lock.yaml` (the `drawnui-react` pin), the removal of
    `src/drawnui/`, `scripts/generate-catalog.mjs`, `scripts/sync-drawnui.mjs`, `catalog/*`,
    `src/render/*`, `src/drawnui-runtime.ts`, the imports in `src/editor` and `src/gallery`, `public/`
    and `reference/` refreshed from the upstream tag, `README.md`, `docs/CATALOG.md`, `tsconfig.json`
    and the Dockerfile if its install step needs it;
  - fixes to whatever the fiddle turns up, with tests, and `specs/future.md` entries for what is
    deferred;
  - the `v0.4.0` release.
- nxlang.org/playground draws with DrawnUI preview.12 once `main` is deployed.
- Existing NX shares on drawfiddle.com stop drawing in the player once the upgraded runtime is
  deployed. They report the schema mismatch instead, and their source still opens in the editor.
