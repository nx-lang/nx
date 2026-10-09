## Context

See `proposal.md` for the motivation. What shapes the approach:

- **The fiddle's language contract** (`docs/ADDING-A-LANGUAGE.md` there). A language has one
  surface. `nx` is on the React surface. The Frame surface exists for "an engine that needs a page
  to itself": `RunAsync` returns a page address, and the engine shows it in a frame. The fiddle's
  Rust language uses it, with a server build behind every run. Since 2026-10-09 that language
  (`RustLanguage`, `fiddle-rust.js`, `rs/run.html`) ships in the engine itself, with the build
  server left to the host.
- **The NX compiler already runs in the fiddle's page.** `window.NxDrawn.compile` turns a snippet
  into an NX IR image linked by name and version against the catalog's own image. The fiddle pins
  `@nx-lang/*` 0.6.0 (0.4.0 when this change began; upstream moved it on 2026-10-09), which writes
  IR schema 5, the schema the Rust runtime on `main` reads. The
  fiddle's 99 NX tests pass against this checkout (`npm run test:nx -- --nx ../nx`).
- **DrawnUI for Rust is statically typed.** A control's properties are builder and setter methods
  declared through its `props!` and `base_props!` macros (`text / set_text: String = ..., MEASURE`).
  There is no way to set a property by name, and no `FromStr` on its enums. Its source is regular
  enough to read mechanically: a script over the catalog and the `drawnui` crate matches most of the
  catalog's 371 properties by name (`MaxLines` to `max_lines`), and 20 of 32 events to an
  `on_<event>` hook.
- **A WebAssembly build has a 1 MiB stack** that layout and paint share with the NX evaluator.
- **Function values are records that name a module-level function.** A templated control's
  `ItemTemplate` is one, so a cell's content comes from `call_function`, not from a component's
  render, and DrawnUI binds cells during layout, not while the host walks the output.
- **The crates are unpublished.** crates.io has no `nx-ir-runtime`, and the workspace version in
  this checkout is `0.1.0`; the release version is written at pack time.

## Goals / Non-Goals

**Goals:**

- Every NX preset of the fiddle draws through `nx-rust`, with its handlers working.
- An edit redraws without a server and without reloading the player.
- What cannot be drawn is named to the author, never dropped silently.
- The runtime API is exercised widely enough to publish with confidence: the instance tree, the
  stack budget, function values, limits and diagnostics on a real engine.

**Non-Goals:**

- Pixel parity with `nx`. DrawnUI for Rust and DrawnUi.React differ in ways their own parity
  document lists; the player reports unsupported properties, it does not emulate them.
- A project export, publishing, or a native (non-browser) player.
- Tracked reads or a runtime-owned tree (`specs/future.md`). The player reconciles by walking, as
  the spike does.
- Changing `nx`. Where the two languages differ in behaviour, this change records it.

## Decisions

### A second language, not a switch on `nx`

`nx-rust` is a language entry of its own, on the Frame surface, registered by the host next to `nx`.

A share stores its language id, and the engine picks the player from it. One language with a
renderer switch would need a second axis in shares, presets, `window.fiddle` and the player, all in
engine code. A second entry needs none of that, and the engine's rule that switching to a language
keeps a buffer that already holds it means moving between `nx` and `nx-rust` keeps the snippet.

Both entries share everything before drawing: the C# descriptor derives the presets, the foreign-code
test and diagnostics from `NxLanguage`, and `fiddle-nx-rust.js` calls the compiler `fiddle-nx.js`
already loads.

### The player is a page that stays loaded, fed over `postMessage`

`RunAsync` always returns the same address, `nx-rust/player.html`. The engine replaces a frame only
when its address changes, so the frame, the WebAssembly module, its fonts and its GPU context
survive every edit.

The image travels by message:

| From | Message | Meaning |
|---|---|---|
| frame | `ready` | the module is up; the page answers with the latest image |
| page | `image { id, bytes, background, gestures }` | draw this |
| frame | `result { id, ok, errors }` | the image linked and rendered, or why not |
| frame | `console { kind, line }` | a snippet's print, an unsupported property, an inert handler |

`RunAsync` compiles, posts the image and awaits the `result` with the same `id`, so a link or render
error appears under the editor like a compile error. The page keeps the latest image, because the
frame does not exist yet on the first run: it asks when it is up. Messages are accepted only from
the frame's own window, and the frame accepts them only from its parent.

Because nothing is compiled on a server, `AutoRun` stays on: HotReload works as it does for `nx`.

*Alternative: a new address per run.* That is the contract's default and what the spike's first page
did. It reloads 6.4 MB of WebAssembly and loses the GPU context on every keystroke pause.

### State survives an edit only where the spike kept it

A new image is a new program. The player links it, evaluates `root`, and reconciles the controls in
place where the type at a position is unchanged, so a scroll position survives, as the spike showed.
Component state does not survive an edit: the instance tree belongs to the program it was made for.
This is what `nx` does on a re-run.

### The player lives in the fiddle repository

`nx/rust/` holds the crate, next to the TypeScript renderer it is the counterpart of. Everything
DrawnUI-specific about NX already lives in the fiddle (the catalog, its generator, the renderer);
this repository keeps no concept of DrawnUI.

*Alternative: DrawnUI for Rust's repository.* The player is a DrawnUI app, but it is also bound to
the fiddle's catalog and message protocol. *Alternative: this repository.* It would bring DrawnUI,
Skia and Emscripten into NX's CI.

### The control table is generated, and committed

`nx/rust/generate-controls.mjs` reads `catalog-meta.json` and `drawnui.nx` on one side and the
`drawnui` crate's source on the other (the `props!`, `base_props!` and `setters!` blocks, the enums
behind the catalog's unions, the `on_*` hooks). It writes `nx/rust/src/controls.rs`:

- one constructor per catalog component;
- per component, a by-name setter used at build and at patch, and a by-name reset that writes the
  default the `props!` block declares;
- per union, a match from case name to enum variant;
- per event, the hook and the action it builds from the parameter names the catalog records.

Conversions are a short hand-written table from the catalog's type to the Rust type (`float64` to
`f32`, `i32` or `usize`; `string` to `String`, `Color` or `Option<Color>`; a record to `Thickness`,
`CornerRadius`, `SkiaShadow` and the rest). Names that differ are a second hand-written table
(`Type` to `layout_type` or `shape_type`, `HoverChanged` to `on_hovered`), and a property with no
counterpart goes into `nx/rust/unsupported.json` with the reason. The generated file and the list
are committed, like the catalog, so a DrawnUI bump shows as a diff. The build refuses to run when
the generated file no longer matches the catalog version or the `drawnui` version it records.

*Alternative: write the table by hand.* 371 properties across 35 controls, rewritten at every
DrawnUI preview. *Alternative: a by-name API in DrawnUI for Rust,* generated by its own macros. That
is the better home in the long run, and worth proposing upstream once the generator shows what such
an API has to carry; it is not ours to add now.

The generator reads the crate from the cargo registry copy the build resolved, so it sees the
version the player links, not a sibling checkout that may differ.

### Reconciliation: walk, patch in place, reset what was removed

The player walks the rendered value as the spike does. At each position: the same control type is
patched (changed properties set, removed properties reset to their default, children reconciled by
position); another type is replaced with `replace_child`. Authored components go through the
`InstanceTree`: `begin`, a `visit` per descriptor keyed by its path, `finish`. Every pass walks
the whole render, and only the properties whose values changed reach a setter. The player does not
ask `is_settled` to skip unchanged subtrees: measured on the presets, the comparison is a twentieth
of what an event costs, the rest being the dispatch and the component rendering its body.

Handlers: the walk gives each handler record the key of the node it was found under. An event calls
`dispatch` with that key; a `HostEffect` comes back for an action nobody bound, and the player prints
the fiddle's `Print` action and reports any other, as `nx` does.

### Cells are held in a tree of their own

DrawnUI for Rust drives a templated control through `items(count, template, bind)` and calls `bind`
during layout, for the cells it realizes. `bind` calls the template function with `Item` and `Index`
and builds the result inside the cell.

A component in that result cannot be a node of the drawing's tree: it is not visited during a pass,
so the next `finish` would drop it. The player keeps a second `InstanceTree` over the same program
for cells and never runs a pass on it. A cell's component is visited under no parent at a key made
of the list's path and the item's index, and removed by key when the list leaves the drawing or its
collection changes.

So a component in a cell keeps its state while its list stands, including while it is scrolled out
of view, and handlers bound in its own body run. A handler the template passed in has no owner and
is reported inert, as the tree already does for a handler under no parent.

This differs from `nx`, whose cells initialize a component afresh at every bind. The difference is
recorded in the fiddle's `README-NX.md`; bringing `nx` to the same behaviour is a follow-up there.

No requirement of the instance tree changes for this. A test here pins the use (visits with no pass,
a dispatch, a removal by key), and the runtime README gains a section on it, because it is the first
thing a host with a virtualized list will ask.

### Stack and limits

The player sets `max_stack_bytes` from `emscripten_stack_get_free()` less a 32 KiB margin at each
call into the runtime, as the spike does, so a snippet that recurses too deep ends with
`nx-ir-resource-limit` under the editor and the page stays alive.

### The player's toolchain and where it is built

`npm run runtime:rust` builds the player into `<web root>/nx-rust/` (the page, the module with the
catalog image compiled in, DrawnUI for Rust's host script, and the fonts of the Rust export
template the engine already ships). It is separate from `npm run runtime` because it needs Rust with
`wasm32-unknown-emscripten` and Emscripten 6, and a first build compiles for minutes. The output is
not committed. A host that offers `nx-rust` runs it; one that does not is unaffected.

The NX crates declare `rust-version = "1.98.1"`. The player is built with that toolchain, which
DrawnUI for Rust (edition 2024) accepts, so the open question about lowering NX's minimum does not
block this change. It stays open for an app that adds NX to a DrawnUI project on an older toolchain.

### Path dependencies until the release

The player's manifest names `nx-ir-runtime` and `nx-value` by path (`../../../nx/crates/...`) while
this change is under way, so a fix here is one rebuild away from the fiddle. The fiddle branch is
not merged in that state. The last task group releases the crates and replaces the paths with the
published version; the branch merges after that.

*Alternative: a git dependency on a commit of this repository.* It would let the branch merge
before the release, but every fix here would then need a push and a new pin.

### A share plays from its artifact

`ShareArtifactAsync` returns the module `nx` returns: the snippet's own image, by name and version
against the catalog, under the 256 KB limit. In the `/p/` player the language draws it without the
compiler: the page's script reads the share id the engine already exposes (`fiddlePlayId()`),
fetches the stored module as the React player does, and hands its program to the frame. The
engine's contract is unchanged.

The player page compiles the catalog image in, so a share made before a catalog change meets the
version check the runtime already does and fails with its message, as `nx` does.

A `/p/` link does not start .NET either. The host's page asks the script whether the share is an
`nx-rust` one, after the React surface has said it is not its own, and on a yes the script makes
the frame itself, hands it the stored program, adds the badge and tells an embedding page `ready`.
That is how `nx` and the engine's Rust language play their shares. The React check leaves the
module response it fetched where the script can take it, so the backend is asked once. A host that
does not add the two lines plays the share through the page, which still never loads the compiler.

### What the page reads once goes into its address

DrawnUI for Rust reads the gestures mode when its canvas starts. The fiddle's Lock mode is
therefore `player.html?gestures=lock`. The engine takes a new address at a user run or a run of
changed code, so the mode reaches the frame at the next such run; an edit alone never reloads it.
The canvas background is sent with each program, so it too follows at the next run.

### What building it changed

Found while implementing, and now part of the design:

- **A control is built with its children.** DrawnUI for Rust lets a container gain and lose children
  while mounted, but a scroll takes its content only when it is built. So controls are built bottom
  up, and a control that is not a container is replaced when the controls inside it change.
- **An event's render is drawn after its handler returns.** While a handler runs, DrawnUI holds the
  control it belongs to outside the tree, and no setter reaches it. The dispatch runs at once; the
  drawing is done by a timer of no delay, before the next layout.
- **Properties are set after mounting, by one path.** Build and patch both go through the typed
  setters, so there is one conversion per property and a reset is the same call with the default.
- **Handlers are read at dispatch time, drawing waits.** A dispatch retires the tokens of the
  output it replaced. With the drawing deferred, a second event in the same frame (a scroll
  reports several) named a retired token and was refused. The controls that stand now take the
  new render's tokens as part of the dispatch; a control the drawing will replace loses its own.
- **DrawnUI's value conventions are the catalog's.** A `CornerRadius` with only its first corner
  rounds every corner, and an eight-digit color is `#AARRGGBB`. Both were found by drawing the
  presets beside the React renderer.
- **A list's items are not walked.** The drawing pass copies a collection without looking into it:
  what is in an item is drawn by the template, in a cell.
- **Two properties cannot be reset** (`SkiaDecoratedGrid`'s line gradients): their part's module is
  private and their default calls a private function. They are listed, and worth an upstream ask.
- **A new program is drawn over the controls that stand.** The first version removed every
  control at each load, to be rid of cells showing the old program's items, and so lost the scroll
  position the spec promises an edit keeps. Now the controls are compared with the new render as
  after a dispatch; what is forgotten at a load is every handler and every list, so each control
  is bound again and each templated control binds its cells against the new program.
- **The player bounds its own depth.** The runtime's limits are per call and every visit is a
  call, so a component that draws itself met none of them and overran the stack, taking the tab
  with it. A pass now fails past 128 nested controls and components, and each visit is given the
  stack that is free where it is made.
- **The runtime bounds the tree too.** The review asked whether a self-rendering component should
  be every Rust host's problem to find. It should not: `InstanceTree::visit` now refuses a node
  more than `max_component_depth` instances deep, 100 by default and the host's to set, with
  `nx-ir-resource-limit` naming `maxComponentDepth` and the component. The default is the
  call depth's, since a component that renders itself is the component form of a function that
  calls itself. It is settable because what a level costs in stack is the host's walk's, which
  the runtime cannot see. The player's bound stays: the runtime counts instances and never sees
  controls, and a snippet can nest layouts with no component between.
- **A cell is as deep as the list it is in.** DrawnUI binds a cell during layout, and a cell's
  components are roots of the cell tree, so both bounds started again in every cell and a template
  that draws a list of itself overran the stack. The walk now leaves its depth on each templated
  control it meets, and a cell's walk goes on from there: the cell past the bound fails as a
  failing template does, and the rest stands. The runtime cannot do this for a host. It does not
  know that one root is drawn inside another.
- **What a control cannot do is said by the patch, not the build.** Controls stand across
  programs, so a report made only when a control is built was made once per control. A property
  the control did not take is not remembered, and what the build found (no such control, an event
  with no hook, more controls inside than it holds) is kept on the control and said again, once
  per program.
- **The page's own runs do not start the program again.** The engine runs a Frame language's
  code again after every Run and when an edit is undone. Each program handed to the player starts
  from its initial state, so the script hands over nothing when the run is not the user's and the
  live player already holds that program with that background.
- **A control that is left out leaves an empty place.** The walk turns a `RefreshIndicator` into
  nothing wherever it meets one, also where a component rendered it, and the drawing counts a
  control's places by the controls among its children, not by position.
- **The script asks the frame what is in it.** The engine keeps one frame for all Frame languages,
  and a player can stop. The script reads the frame's address before trusting it, the page says
  when its module ends, and a first run that the frame then rejects is run again so that the error
  is listed under the editor.
- **The module is 6.4 MB, 1.76 MB compressed**, against the spike's 4.78 MB: the table links every
  control.

## Risks / Trade-offs

- **[One function is added to the React surface]** → `fiddleReactCompile` compiles with a registered
  language and draws nothing. Without it the script would need NX's compiler by another route.
- **[Reading DrawnUI for Rust's source is fragile]** → The generator fails loudly on a block it
  cannot parse, records the version it read, and the build checks that version. The declarations
  have one form across 30 blocks today.
- **[68 properties and 12 events are unsupported for at least one control]** → Listed, with reasons, and told to
  the author once per snippet. Some are name differences the second table will close; the rest are
  DrawnUI for Rust's gaps.
- **[6.4 MB of WebAssembly, 1.8 MB compressed]** → Loaded on first use of `nx-rust` only, cached after, and never by a
  visitor who stays in another language. Size is measured and recorded in the build's output.
- **[Performance on a real GPU is unmeasured]** → Headless Chromium draws in software. One task
  measures frame time and the time from image to first frame in a desktop browser.
- **[Two NX renderers can drift]** → The same presets run through both in tests, and known
  differences are written down in one place.
- **[The private host]** → drawfiddle.com registers languages in code this change cannot see. The
  dev host proves the integration; the maintainer adds the same two lines there.
- **[A fix here after the fiddle work starts]** → Expected, and the reason for the order. Each one
  lands in this repository with its own test before the release tasks.

## Migration Plan

1. Fiddle branch from `origin/main`, built against this checkout by path.
2. The player, the generator and the language entry, to the point every NX preset draws.
3. Fixes here, on `main`, as they are found.
4. Release the crates (`v0.7.0`): the user pushes the tag and publishes the draft. This also closes
   the two tasks `support-rust-hosts` left open.
5. The fiddle moves to the published versions and its pull request is opened.

Rollback before step 4 is deleting a branch. After it, the crates are permanent and a fix is a new
version.

## Open Questions

- Whether drawfiddle.com serves `nx-rust` at all, and from which build. The maintainer's decision;
  nothing here depends on the answer.
- Whether `nx` should hold cell components the way `nx-rust` will. A follow-up in the fiddle.
