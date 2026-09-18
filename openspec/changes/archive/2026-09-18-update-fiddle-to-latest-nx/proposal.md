## Why

The DrawnUI fiddle (`~/src/DrawnUi.FiddleEngine`, branch `nx-linkable-ir`) was last brought up
against NX when schema 3 had just become a binary image. Since then this branch of NX
(`enhance-nx-fiddle-support`) has moved the format on to schema 4 (the `actionHandler` node, `emits`
on component declarations, the conversion nodes), taught the TypeScript IR runtime to initialize instances and dispatch
handlers, and made primitive conversions exact (`"Total: " + count`, `int` at a `float64` site,
narrowing rejected). None of it has reached the fiddle: its image rework is uncommitted, its
committed catalog image predates the amended layout, its catalog still omits every DrawnUI event,
its renderer expands an authored component once and never dispatches, and its four presets are
static ports in which nothing answers a tap. The fiddle is the public DrawnUI playground for NX, so
it should show what NX can do now, and running it against this checkout is the last end-to-end
check of these NX changes before they are tagged and published.

## What Changes

- The fiddle branch is brought up to date first: `origin/main` is fetched and merged into
  `nx-linkable-ir` (at the time of writing `main` holds nothing the branch lacks, so this is
  expected to be a no-op, and is checked rather than assumed), and the uncommitted image rework on
  the branch is committed so the work below starts from a clean tree.
- Everything is built and tested against this checkout in side-by-side mode
  (`npm run runtime -- --nx ../nx`, `npm run test:nx -- --nx ../nx`) after `pnpm install &&
  pnpm -r build` here. The `@nx-lang/*` pins in the fiddle's `package.json` stay at the published
  `0.1.0` until the NX release is tagged; moving the pins, and the pull request, follow the release
  and are not part of this change.
- The binary, per-module artifact path is finished against the current format: one `.nxir` image
  per module from `generateNxIr()`, the snippet's image alone in a share, and the catalog image
  bundled with the runtime and linked by name. `nx/catalog/drawnui.nxir` is re-emitted with the
  current compiler (its layout now carries `emits`), and every remaining reference to the JSON
  encoding (`drawnui.nxir.json`, the docs, `modules.d.ts`) goes.
- The fiddle's catalog generator declares every DrawnUI event as an emit, by the rules the NX
  playground's generator already follows: the emit sits on the component for the class that declares
  the event, a callback parameter with a primitive NX type becomes a payload field, a parameter
  typed as an engine object is dropped and recorded, and `catalog-meta.json` records each event's
  parameter names. Event callbacks leave `omitted.json`; functions that are not events stay there.
- The fiddle's renderer draws every use of an authored component as a runtime instance, held in an
  instance tree keyed by position. A handler record on a control becomes that control's DrawnUI
  callback; the callback dispatches one batch against the instance whose body bound the handler
  and redraws. Updates patch the owner's state, an emitted action reaches the handler the parent
  bound, child state survives a parent redraw, and a failed dispatch leaves the drawing as it was.
  Every runtime report goes to the console pane under the editor, through the channel a TSX
  snippet's own `console.log` already reaches it by, and to the browser console, which is all the
  `/p/` player has; a handler bound in `root`, outside any component, is drawn inert and reported
  once the same way. Because NX has no statement that prints, the host reads one action as a print:
  an unhandled `Log { text }` is written out as its text alone, which is how a snippet answers the
  `Console.WriteLine` and `console.log` its C# and TSX twins use.
- The share path is unchanged in shape and gains interaction: the player mounts a share's image,
  builds the same instance tree, and handlers run in `/p/` without the compiler.
- The NX presets stop dropping what NX now has. Welcome's button counts its taps and says
  `Button clicked N time(s)` in the console, the line its C# and TSX twins print (state, a handler,
  an emitted action, a number in a string). Text and Shapes are re-ported from the playground's current, complete
  versions (a tapped span and a tapped markdown link reported in a label; a `ContextMenu` handler).
  Layouts keeps its static demos and is tidied wherever the new conversion rules allow. Two presets
  are new: Controls, ported from the playground's Common Controls example, shows switches,
  checkboxes, sliders and buttons reporting into a label through emits from a child component to
  its parent; Cards is the twin of the engine's own TSX Cards template, drawing a list of records
  with a `for` that binds each plan and its index. The component a preset draws is named `App`, as
  the React templates name theirs. Preset descriptions and `nx/CATALOG.md` say what is interactive.
- Tests grow with the code: generator tests for emits, an instance-tree test (the playground's
  cases), runtime tests that compile every preset, dispatch a handler against each interactive one
  and check the redraw, and the share-size budget. The change ends with the whole thing run in a
  browser: the dev host, every preset, taps, a share round trip and the `/p/` player, headless
  through `window.fiddle`.
- Whatever the fiddle turns up in NX itself is fixed on this branch, with a test, before the
  release; the fiddle is not worked around.

Out of scope: tagging the NX release, repinning the fiddle to published versions and opening its
pull request; the engine's C# and TSX languages; porting the playground's remaining examples;
showing host effects in the fiddle's own UI.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `editor-language-service`: completions and hover cover `on<Emit>` handler properties, which the
  fiddle's browser run found missing (the one NX defect this change turned up, besides stale
  expectations in the wasm SDK's tests).
- `fiddle-nx-language`: the catalog the fiddle derives declares DrawnUI events as emits; authored
  components are drawn as instances and their handlers run, in the editor and in the player; the
  preset requirement names interactive presets; the runtime build's checkout mode becomes a stated
  requirement rather than a convenience.

## Impact

- `~/src/DrawnUi.FiddleEngine` (branch `nx-linkable-ir`), which is where nearly all the code lands:
  - `nx/generate-catalog.mjs`, `nx/catalog/{drawnui.nx,drawnui.nxir,catalog-meta.json,omitted.json}`,
    `nx/CATALOG.md`: events as emits.
  - `nx/src/instances.ts` (new), `nx/src/draw.tsx`, `nx/src/values.ts`, `nx/src/runtime.ts`: the
    instance tree, handler records to callbacks, the mount holding a tree per program.
  - `nx/test/*.test.ts`: instance tree, dispatch through presets, generator emits.
  - `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`: the presets.
  - `README.md`, `AGENTS.md`, `dev/build-nx-runtime.mjs`, `nx/nx-packages.mjs`: docs and any
    checkout-mode fix the run turns up.
- This repository: `openspec/specs/fiddle-nx-language` and `editor-language-service` are updated;
  `crates/nx-language-service` lists a component's emits as `on<Emit>` properties; the wasm SDK's
  tests expect schema 4 and a compile failure that the new conversion rules still reject.
- No published package, version or pin changes.
