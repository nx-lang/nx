## Context

See proposal.md for motivation. What shapes the approach:

- Two repositories. The code lands in `~/src/DrawnUi.FiddleEngine` on branch `nx-linkable-ir`; this
  repository holds the change, the `fiddle-nx-language` spec and whatever NX fix the work turns up.
  The NX repository keeps no DrawnUI-specific code for the fiddle, so nothing here is imported by
  the fiddle except the published `@nx-lang/*` packages.
- The fiddle branch has an uncommitted rework for binary images (share is `const IR = "<base64>"`,
  bundled `nx/catalog/drawnui.nxir`, a base64 esbuild loader) whose tests passed against this
  checkout on 2026-09-16. The format has since moved to schema 4 (`ir-action-handlers`, then the
  conversion nodes), so that image is stale by construction, and the build's staleness check
  says so.
- `origin/main` of the fiddle is already an ancestor of `nx-linkable-ir` as of 2026-09-17.
- Side-by-side mode exists: `nx/nx-packages.mjs` resolves every `@nx-lang/*` import, the wasm
  module and the type declarations from a checkout given by `--nx <dir>` or `NX_CHECKOUT`.
- The NX playground (`sites/playground`) already solved the two hard parts for the same DrawnUI
  package: `scripts/generate-catalog.mjs` declares events as emits, and `src/render/instances.ts`
  with `values.ts` composes runtime instances into a tree and binds handler records to callbacks.
  The fiddle's generator and renderer are older forks of the same files.
- The fiddle draws through the page's React (`window.React`) inside a component the engine mounts
  from the compiled module's default export. It has no pane for runtime reports; NX runtime
  problems go to the console today.
- A fiddle snippet's top-level element compiles to the `root` function, which is pure evaluation:
  handlers bound there carry no token and cannot run. Interaction needs a component.

## Goals / Non-Goals

**Goals:**

- The fiddle runs, tests and draws against this checkout with nothing published.
- Handlers, state and emits work in the editor and in the player, with the same routing the
  playground implements and the playground's tests to prove it.
- Presets that exercise those features and are compiled and dispatched by the fiddle's tests.
- Any NX defect found is fixed in NX, so the release that follows is one the fiddle has already run.

**Non-Goals:**

- A shared package for the generator or the instance tree.
- A UI in the fiddle for host effects or dispatch failures.
- Changing the engine's language contract (`compile`, `prepare`, the share module's shape).
- Repinning, tagging, or the pull request.

## Decisions

### 1. Port the playground's generator changes and instance tree into the fiddle; share nothing

The fiddle's `nx/generate-catalog.mjs` takes the playground generator's event handling (event
detection, payload fields from primitive parameters, dropped parameters recorded, `events` with
parameter names in `catalog-meta.json`). `instances.ts` is copied as `nx/src/instances.ts` with its
test, and `values.ts` takes the handler-record-to-callback binding, including the `ContextMenu`
convention the playground uses.

Alternative considered: publish the instance tree from `@nx-lang/ir-runtime` or a new package. It
is host-neutral in principle, but it has one and a half callers, the playground is due to drop
DrawnUI, and a new public API would have to be right before the release this change is meant to
de-risk. Copying 330 lines now costs little; promotion can follow once two hosts have used the same
code unchanged. The port should keep the file diffable against the playground's so that promotion
stays easy: change imports and types, not logic.

### 2. The instance tree lives with the mounted component and is rebuilt when the image changes

`NxProgram` (the component behind `mount`) holds one instance tree per mounted program in a ref
from the page's React, keyed to the `ir` it was built for, and a counter in state that a completed
dispatch bumps to redraw. A new image (an edit and a run) discards the tree, which is the spec's
"running again resets the instances". The prepared and linked program stays cached by image string
as today; only instances are per mount, so two players of one share on a page do not share state.

Alternative considered: a tree per prepared program in the existing `programs` map. It would leak
state between mounts and outlive the component.

`ReactLike` grows `useRef` and `useState`. The test stub in `nx/test/support.ts` grows minimal
versions that let a test call the bound callback and render again.

### 3. Reports go to the engine's console pane, and always to the browser console

Host effects, dispatch failures and inert root handlers are reported in the wording the playground
uses, each of the deduplicated ones once per prepared program (the engine mounts one image several
times: a hot reload, the run, React's development double render).

They go to two places. `window.fiddleReactOnConsole` is the channel the engine already installs for
a TSX snippet's own `console.log`, which lands in the console pane under the editor; the runtime
calls it when the page has set it, so NX's lines sit in the author's list next to the other
languages', in order. Every line is also written to the browser console, because the pane belongs to
the editor page and the `/p/` player has none.

The pane is the right home for these: it is where an author already looks for what a snippet said,
and it costs no new UI. The engine's *error list* is a different thing, for compile results
populated by the engine, and is still left alone.

Since NX has no statement that prints, the host also reads one action as a print: an unhandled `Log`
carrying `text` is written out as that text alone, so a snippet can answer the twins'
`Console.WriteLine` / `console.log` line for line. The convention lives in the fiddle, not the
language — it is this host's one host effect, in the same spirit as the `ContextMenu` convention in
`values.ts` — and `Log` is a name the snippet declares itself rather than something added to the
generated catalog.

One event is reported and does not run: the second of a pair fired on one drawing. A dispatch
replaces the instance and retires the handler tokens the drawing carries, so an event delivered
before React commits the next drawing names a handler that is gone. It cannot be dispatched against
the instance that replaced it either, because the tokens are numbered per generation and the new
output may not hold a handler where the old one did. It is therefore dropped — but said, every time
rather than once per program, as a failed dispatch is: it is this tap that did nothing. A radio
group meets it deterministically, toggling the button that was on and the button that was tapped in
one gesture, so the console is where the visitor learns why only one of the two took effect.

A host effect is reported with the component that produced it, not the position of the instance:
the tree's key is a path through a drawing (`root.1.0/body`), which tells a visitor reading the
console less than the component's name does. The spec says the component for the same reason.

Alternative considered: printing every host effect in the descriptive form. It reads as a diagnostic
rather than as the snippet talking, so the starter could not match its twins' output, which is the
point of having three twins.

### 4. Presets are components with a top-level use, named as the twins name theirs

Each interactive preset declares an `App` component holding the state and ends in `<App />`. `App`
rather than the playground's `Page`: in the fiddle the three languages sit side by side and the TSX
templates export `App`, so the galleries read alike. The playground's own examples keep `Page`, and
the two diverge there as they already do over fonts, assets and captions. A preset with no state
and no handler of its own stays one top-level element and declares no `App` — Layouts is the only
one, and its helper components keep their own names.

The playground's complete Text and Shapes ports are already written this way, so they port nearly
verbatim, with the fiddle's fonts and assets substituted as before. Controls is the playground's
`looks.nx`, which is the emit-to-parent example.

Welcome and Cards are twins of presets that exist already, so they follow those rather than the
playground's. Welcome: one `taps:int`, a `titleOf(count)` function both the button and the label
above it read, the twins' own `Click Me!` / `Clicked N times!` captions on a `SkiaButton`, and two
`Log`s per tap. Cards: the three plans as a list of `Plan` records and a `for plan, index in plans`,
which is what the twin's `PLANS.map((plan, i) =>` becomes — the index is what lets the card know
which one it is and the tap name it.

The second `Log` in Welcome is the interesting one. Both twins report the title changing as well as
the tap — C# by observing the button's `Text`, TSX with an effect on its state — and NX has neither
a property observer nor a lifecycle effect. The handler that changes the title says it changed,
which is the same divergence the TSX twin already documents in its own source against C#. It orders
the pair as the C# twin does, whose observer fires inside the setter; the TSX twin's effect runs
after its commit, so the two twins already disagree about the order and NX can only match one.

A preset carries no comments explaining itself. The code is the example, its `description` in the
preset list says what it shows, and the editor's language service answers what a name means — prose
in the snippet only competes with all three, and an author who copies a preset as a starting point
inherits it. The exception is a comment that says something is *missing*: what the example cannot
show because NX or the catalog does not offer it, and what it therefore does instead. Four survive
(no `\n` escape, no pointer position on `ContextMenu`, one event per gesture from a radio pair, no way
to get a string's length), and each stands next to the code it excuses. Provenance stays in C# comments
outside the snippet, where the fiddle's readers never see it.

Known limitation, not worked around: NX cannot express a double quote inside a string literal, so
Welcome's mirror label omits the quotes the twins put around the title. See the note in tasks.md.

Three more are twins of C# templates the gallery already ships, taken because each needs nothing NX or
the catalog lacks: UI (the themed settings screen), Editor (`SkiaEditor`) and Glass (`SkiaBackdrop`).
Each is the same screen with the wiring turned around — the twins observe a control's property or keep
a field to poke, while NX patches the state the control draws from — and Glass, which observes nothing, stays one
top-level element, its four background circles a `Glow` element function taking what differs between
them — a function rather than a component because it expands at the call site, so it needs neither
`extends DrawnNode` nor a block around its body. The rest of the C# gallery stays untwinned on
purpose: Effect wants `VisualEffects`, Cells wants `ItemsSource` and recycling, and neither is in the
catalog; Spinner, Custom, SKSL and Raw want animation, subclassing or a raw canvas; Data I/O and
Weather want the host's `Fiddle` API; and Looks is what Controls already shows. Kids is the near miss:
its helpers become components readily enough, but what makes it Kids is the kid's four lines sitting
*above* the helpers under a banner saying to ignore the rest, and NX puts the element it draws last —
anything after it is a syntax error — so the four lines land at the bottom. The banner is prose in the
snippet besides, which a preset here does not carry, and `Colors.Red` becomes `"#FF0000"`.

The preset list stays short (nine, against the TSX gallery's four and the C# gallery's thirteen); the
fiddle is not a second playground gallery.

### 5. Tests read presets from the C# file and dispatch generically

`runtime.test.ts` already parses `FiddlePresetsNx.cs`. For each preset the test draws through the
real `drawValue`, collects every element prop that is a function, calls the first with stub
arguments, redraws, and asserts the element tree changed. That proves catalog events, handler
binding, dispatch and redraw together without a per-preset script. Presets with no handler are
asserted to have none, so a preset cannot silently lose its interaction.

### 6. Commit the pending rework before anything else, in its own commit

The fetch and merge need a clean tree, and the image rework is a finished, tested unit. It is
committed first (catalog image re-emitted against this checkout in the same commit, since the old
one cannot pass the staleness check). Events, the instance tree and the presets follow as separate
commits so the eventual pull request reads in order. Commits are local; nothing is pushed.

### 7. NX defects are fixed here, test first

If the fiddle fails because of NX (runtime, SDK, Monaco package, compiler), the fix lands on
`enhance-nx-fiddle-support` with a test in the owning package, the checkout is rebuilt, and the
fiddle is re-run. The fiddle gets no workaround for a bug in an unreleased package.

## Risks / Trade-offs

- [The copied instance tree drifts from the playground's] → keep logic identical and port its test
  file; note the origin in the file header; promotion to a package is the recorded follow-up.
- [The fiddle's `drawnui-react` preview differs from the playground's vendored DrawnUI, so event
  detection may meet shapes the playground generator never saw] → the generator records what it
  drops, and the regenerated `omitted.json` and `drawnui.nx` diff is reviewed as part of the task.
- [Emits on every control enlarge the catalog image and the runtime bundle] → measured in the task
  that regenerates it; the share budget is unaffected because a share carries no catalog.
- [Handler-bearing snippets make larger share images] → the 128 KB budget test covers the new
  presets.
- [Pins stay at 0.1.0, so a plain `npm run runtime` without `--nx` fails on this branch until the
  release] → stated in the fiddle's README section for the branch and in the commit message; this
  is the accepted state until the release, and the reason the pull request waits.
- [DrawnUI callbacks may fire during a React commit] → dispatch synchronously, redraw through the
  state counter; if the engine objects to a synchronous set-state, defer the bump with
  `queueMicrotask`. Checked in the browser task.
- [Headless taps on a canvas] → use the DrawnUI accessibility overlay for coordinates, as recorded
  for the playground, or invoke the control's callback through the engine where a tap is unreliable.

## Migration Plan

Nothing deploys. Order: build this checkout, commit the fiddle's pending rework with a fresh catalog
image, merge `origin/main`, then catalog events, renderer, presets, docs, browser run. Rollback is
`git reset` on a local branch. After this change: tag the NX release, repin the fiddle, open its
pull request (tracked outside this change).

## Open Questions

- Whether the instance tree should move into `@nx-lang/ir-runtime` after the release. Does not
  affect this change.
