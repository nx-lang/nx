## Context

The fiddle's NX side lives under `nx/` in `~/src/DrawnUi.FiddleEngine`. Its NX presets are in
`src/DrawnUi.Fiddle/FiddlePresetsNx.cs`. It pins `@nx-lang/*` 0.3.0 and `drawnui-react`
0.1.0-preview.12. Its `main` is level with `origin/main`, and the NX work from earlier changes was
merged there as PR #2. The branches `nx-linkable-ir` and `add-nx-support` are spent.

On the NX side, `occurrence-cardinality` has already migrated everything the fiddle copied from:
- the playground's catalog generator, `sites/playground/scripts/generate-catalog.mjs`, including its
  `CATALOG.md`;
- the playground's examples and its instance-tree tests;
- the `fiddle-nx-language` spec's catalog requirements.

The playground's renderer needed no code change. So the fiddle's work is mostly a port of a
known diff (`git diff v0.3.0..HEAD -- sites/playground`), applied to files that started as copies of
the playground's.

The playground draws with DrawnUi.React source vendored into `src/drawnui/` (776 KB) from
`~/src/DrawnUi.React` at preview.4 by `scripts/sync-drawnui.mjs`. The same script copies the fonts,
images, Lottie files, sprite sheets, shaders and reference demo pages into `public/` and
`reference/`. `docs/CATALOG.md` records no local edits to the vendored tree. The playground
imports:
- `drawnui/index` and `drawnui/react/index`, the package's `./core` and `.` entries;
- `controls/SkiaLabel`, `controls/SkiaLayout` and `core/SkiaControl`, which `./core` re-exports;
- `react/registry` for `Registry` and `applyProps` (in `render/materialize.ts`). Upstream has since
  folded that file into `src/react/reconciler.ts`, and the package's entry points export neither.

Its catalog generator reads tags from that registry. The fiddle's generator reads them from the
package's React entry, which is why the two catalogs differ over `SkiaGrid`. The playground's
`NxTemplateCell` overrides `OnBindingContextChanged`, which the package declares as a protected
member of `SkiaLayout`. The npm package ships `dist` only, with no assets.

The NX branch `refine-sequences-and-optionals` has two commits past `main`
(`c197f4e`, `6f56e76`), is not pushed, and is what "the NX repo here" means below.

## Goals / Non-Goals

**Goals:**
- The fiddle builds, tests and runs in a browser against this checkout, with no NX 0.3.0 spelling
  left in anything it generates, ships or documents.
- Every NX defect the run finds is either fixed here with a test or recorded in `specs/future.md`,
  with a fiddle workaround that names the entry.
- After the release, the fiddle runs on the published 0.4.0 packages with no `--nx`.
- The playground draws with the same `drawnui-react` release as the fiddle, with no vendored runtime
  source, and its catalog is derived the way the fiddle's is.

**Non-Goals:**
- New fiddle features, presets or interaction.
- Playing schema-4 shares.
- Any change to the engine's C# or TSX languages.
- Porting DrawnUI demo pages that are new since preview.4 to the playground.
- Sharing one catalog generator between the two repositories. They stay two copies of one derivation
  and are compared, not linked, because the NX repository keeps no dependency on the fiddle.

## Decisions

### Branch from the fiddle's `main`, not from an old NX branch

The new fiddle branch is `nx-occurrences`, taken from `origin/main` after a fresh fetch.
`nx-linkable-ir` is merged history, so reusing it would only add a merge. The preview.12 catalog
the maintainers regenerated is on `main`, so starting there is how "the catalog's most recent
changes" are taken in.

If `drawnui-react` has moved again when work starts, the pin moves first, in its own commit,
regenerated under 0.3.0. That keeps the package diff and the spelling diff apart for review.

### Port the playground generator's diff; do not re-derive

The fiddle's `generate-catalog.mjs` began as a port of the playground's and still mirrors its
structure. `occurrence-cardinality` changed about 28 lines there: `x?: T` in `nxComponent` and in the
record fields, `+` for arrays and for `TItem`, `content Children?: DrawnNode+`, and the unparenthesized
optional function type. Porting that diff, in place of rewriting against the spec, keeps the two
generators easy to compare. The spec's scenarios, which already name the target spellings, are the
check.

The fiddle-only parts stay as they are:
- the `drawnui-react` package as the source;
- `string | GridLength[]`;
- the emit rules;
- `catalog-meta.json`.

A DrawnUI array becomes `x?: T+`, not `x: T*`. An empty TypeScript array is absence to DrawnUI, and a
property admits zero only through `?` on its name. That is already the spec's rule.

### Presets keep their shape, with only the spelling changed

Each preset gets the smallest edit that compiles under the new rules:
- `content Children:DrawnNode[]` becomes `content Children:DrawnNode+`, as the playground's examples
  and instance tests wrote it. A `Card` with no children never occurs in a preset.
- `colors: string[]` becomes `string+`.
- The one ternary becomes `if … { … } else { … }`.

A preset that needs anything beyond that is exactly what the run should surface. It is handled under
the NX-issue rule below, not by redesigning the preset.

### A schema-4 fixture is captured before anything moves

`node_modules` holds 0.3.0 now. The first step, before any upgrade, compiles one preset with the
installed packages. The resulting share string becomes a test fixture (`nx/test/fixtures/`). The
player-side test then mounts it with the upgraded runtime and asserts that the failure label names
schema 4 and schema 5.

The runtime already rejects a foreign schema with `nx-ir-schema-version`, and `runtime.ts` already
turns a preparation failure into the failure label. So the scenario is expected to hold with no new
code, and the test proves that it does.

Alternative considered: bundle a schema-4 runtime beside the schema-5 one so old shares keep playing.
It was rejected. NX makes no backward-compatibility promise before 1.0, the old bundle would pin a
second compiler's worth of code into every page, and the editor link already recovers the source.

### What counts as "fix now" for an NX issue

An NX defect is fixed on `refine-sequences-and-optionals` when all of these hold:
- the fix is contained to one component: a crate, the IR runtime, or a package;
- a test in this repository can pin it;
- it raises no language-design question.

Otherwise it gets a `specs/future.md` entry. The entry names the fiddle repro, the code location and
the workaround. The fiddle works around it with a comment that names the entry's heading.

A fix lands in this repository first, with its test. The fiddle then picks it up through `--nx ../nx`
after `pnpm -r build`, and the wasm build when a crate changed, so a fiddle failure is never
blamed on a stale checkout build.

### The playground depends on the package and stops vendoring it

`sites/playground` gains `"drawnui-react": "<the fiddle's pin>"`, exact, and `src/drawnui/` is
deleted. Every import moves to `drawnui-react` or `drawnui-react/core`.

Alternative considered: re-run `sync-drawnui` against the preview.12 tag and keep vendoring. It was
rejected for three reasons:
- the reason for vendoring ("a private, unbuilt package") no longer holds;
- nothing edits the copy;
- keeping it would keep the second derivation of the catalog.

### `materialize.ts` builds from exported classes, as the fiddle's cell builder does

`Registry` maps a tag to its class, and the package already exports each control class under its
tag's name from `./core`. So `materialize.ts` looks the class up there, the way the fiddle's
`construct` does, and reports an unknown tag once.

`applyProps` is replaced by what it does that the playground needs. The upstream source at the
preview.12 tag is read first to see whether it does more than assign by name (style application,
event wiring, special props). Anything more is reproduced in `values.ts` or `materialize.ts`, with a
test, not guessed at.

`templateCell.ts` stays as it is, since its hook is part of the package's declared surface. The
fiddle's `SkiaDynamicDrawnCell` approach is the fallback, used only if typecheck or a test says
otherwise.

### The playground's generator becomes a port of the fiddle's

This port runs after the fiddle's generator has the new spelling (section 2), so it takes the
finished rules. The playground keeps its own output paths (`catalog/skia.nx`,
`catalog/catalog-meta.json`), header and `docs/CATALOG.md`. `catalog-meta.json` gains the `version`
field. A playground test compares it with the pin and fails naming both versions and
`pnpm run generate-catalog`, the way the fiddle's runtime build refuses.

With one derivation and one version, the two generated catalogs should match apart from their
headers. `diff` of the two is the check. Any remaining difference is either removed or recorded in
both `CATALOG.md` files. The fiddle's "Differences from the NX playground's catalog" section shrinks
to what is left.

### Assets come from the upstream tag matching the pin

`sync-drawnui` stops copying `src`. It reads the pin from `package.json` and takes the asset and
reference trees from `v<pin>` in the upstream checkout with `git archive`. So the checkout's working
tree and `HEAD` are never touched: the checkout is at preview.4 today. It fails naming
`git fetch --tags` when the tag is missing, and writes `docs/UPSTREAM.md` with the tag and commit.

The playground and the fiddle serve assets independently, so nothing forces the two to hold the same
files. The tag rule keeps the examples' fonts and images in step with the DrawnUI code that draws
them.

### Release order, each outward step on the user's go-ahead

1. Merge the NX branch into `main` (a PR). This also deploys the playground on preview.12.
2. Push the `v0.4.0` tag. The tag alone sets every package's version.
3. Verify the draft release's artifacts.
4. Publish the release, which is what triggers npm and NuGet.
5. Wait for the npm packument to list 0.4.0.
6. Repin the fiddle and run `npm run catalog`. It must reproduce the committed image byte for byte;
   if it does not, the checkout and the release differ, and that is investigated before going on.
7. Run `npm run test:nx` without `--nx`.
8. Push `nx-occurrences` and open its PR.

The PR tells the fiddle's maintainers that deploying it retires existing NX share links, which then
show the schema-mismatch label.

## Risks / Trade-offs

- [The fiddle's maintainers commit to `main` again, for example another catalog regeneration.] → Merge
  `origin/main` before repinning. Resolve a conflict in `nx/catalog/*` by regenerating, never by
  hand-merging generated files.
- [The new spelling changes the share-image size.] → The 128 KB budget test runs over every preset.
- [An NX defect is found only in the browser, after the unit suites pass.] → The headless browser run
  (every preset, taps, a share round trip, the `/p/` player, the schema-4 fixture as a share) comes
  before the release tasks, not after.
- [Existing NX shares on drawfiddle.com break when the upgrade is deployed.] → The player reports the
  break instead of failing silently, the source survives, and the PR says so. The number of NX
  shares in the wild is small, since NX support reached the fiddle's `main` on 2026-09-21.
- [DrawnUI preview.4 → preview.12 changes how a gallery example draws.] → Before the pin moves,
  screenshots are taken of every gallery entry. Afterwards each entry is compared with its
  screenshot, and a difference is either explained by an upstream change or fixed in the example.
- [`drawnui-react` declares `playwright-core` and `vite` as peer dependencies, so pnpm may install or
  warn about `playwright-core` in the playground and its Docker image.] → Check the install and the
  image. If it is pulled in, mark it as an ignored optional peer in the workspace's pnpm settings,
  rather than carrying it into the runtime stage.
- [`applyProps` does something the replacement misses, such as applying a style or wiring an
  event.] → Read it at the pinned tag before replacing it, and cover what it does with
  `materialize.test.mjs`.
- [The published packages differ from the checkout the fiddle was tested against.] → The
  byte-identical `npm run catalog` check after repinning, and the full suite without `--nx`.
