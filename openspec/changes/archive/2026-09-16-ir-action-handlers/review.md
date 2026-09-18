# Review: ir-action-handlers

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/nx-ir-format/spec.md,
specs/typescript-ir-runtime/spec.md, specs/drawnui-nx-catalog/spec.md, specs/playground/spec.md  
**Reviewed code:** the working-tree diff against `3b22745`:
- **Rust:** crates/nx-codegen (builder.rs, emit.rs, ir.rs, ir_image.rs, ir_explain.rs, model.rs,
  lib.rs, ir_tests.rs, ir_corpus_tests.rs, Cargo.toml), crates/nx-ffi/tests/ffi_smoke.rs
- **Bindings:** bindings/dotnet/tests/NxLang.Sdk.Tests/NxEndToEndTests.cs, bindings/node/test/sdk-node.test.ts
- **Corpus:** specs/ir-conformance (README, manifest, new `handlers` program, regenerated images)
- **TypeScript runtime:** runtime/typescript (src/index.ts, tests, README, package.json)
- **Playground:** sites/playground (generate-catalog.mjs and the generated catalog, docs/CATALOG.md,
  src/render/instances.ts and its test, values.ts, DrawnTree.tsx, useNxDrawing.ts, evaluate.ts,
  src/editor/EditorView.tsx, app.css, scripts/expand-components.mjs and check-examples.mjs,
  src/examples/types.ts, examples.json, every changed example, README)
- **Docs:** docs/nx-ir-format.md, docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md, docs/drawn-ui-proposal-review.md

**Checks run:**
- `cargo test -p nx-codegen`: 135 passed.
- `cargo test -p nx-ffi --test ffi_smoke`: 39 passed.
- `dotnet test --filter GenerateNxIr`: 5 passed.
- `pnpm test` in runtime/typescript: passed, corpus lifecycles included.
- `pnpm test` in sites/playground: 72/72 passed, with `check-examples` 20/20.
- `pnpm run typecheck` in sites/playground: clean.
- Catalog generator run twice in a scratch copy: byte-identical to the committed catalog.

Several findings below were confirmed with scratch repro scripts against the real wasm compiler;
none were added to the repository.

## Findings

### ✅ Verified - RF1 A handler that returns two emits bound by the same parent fails part way through routing
- **Severity:** High
- **Evidence:**
  - `sites/playground/src/render/instances.ts:237-244` routes every effect of a dispatch as soon as
    that dispatch returns.
  - `#route` (`:263`, via `creatorOf`) captures the parent's token from the parent's *current*
    instance at that moment.
  - The queue then dispatches the first routed entry (`:232-236`), which replaces `node.instance`
    with a new generation, so the second entry's token no longer exists.
  - Repro:
    - Child: `onTapped={<Child.A /> <Child.B />}`.
    - Parent: `<Child onA=<Update .../> onB=<Update .../> />`.
    - Result: `Unknown handler token 'h1-2' for the 'Page' instance`, thrown after Page already
      applied the first update.
  - The language allows a handler to return several actions (the runtime's `Counter` does
    exactly that).
- **Recommendation:**
  - Batch the routed effects that target the same owner node into one `dispatchComponentActions`
    call; the runtime already accepts several invocations per batch and reads state live across
    them.
  - Alternatively, resolve each pending entry's token at the moment it is dispatched, not when it
    is queued.
  - Add an `instances.test.mjs` case for it.
- **Fix:** `InstanceTree.dispatch` now dispatches the pending node farthest from the root first, with every entry pending for it in one `dispatchComponentActions` batch. Effects only route to strict ancestors, so no node is dispatched twice in a chain. Added `instances.test.mjs` "a handler that emits two actions its parent bound patches the parent with both" (the second handler reads what the first left). D12 describes the ordering.
- **Verification:** `dispatch` now takes the deepest pending node each round and sends every entry for it in one batch. Every routed entry targets a strict ancestor of the node just dispatched, so all pending nodes lie on one ancestor path. Each round moves strictly closer to the root, so no node is dispatched twice, and a queued token cannot be retired before it runs. The new two-emit test passes, and D12 describes the ordering.

### ✅ Verified - RF2 A dispatch that fails mid-chain leaves the drawing's callbacks pointing at replaced instances
- **Severity:** Medium
- **Evidence:**
  - `instances.ts:235-236` mutates each node as the chain proceeds.
  - On a throw, `useNxDrawing.ts:162-164` records only `failure` and does not redraw.
  - Repro:
    - Child: `{<Update c={c+1}/> <Child.A/>}`; the parent's `onA` divides by zero.
    - The child is left at `c=1`, generation 2, while the drawn callbacks still carry generation-1
      tokens.
    - The next tap fails with `The 'Child' instance holds no handler under token 'h1-1'`.
  - The same happens if `draw()` throws after a successful `tree.dispatch`: `begin()` ran without
    `finish()`, and the old drawing again holds stale tokens.
  - Design D12 accepts partial updates, but the spec's "a dispatch that fails SHALL leave the
    drawing as it was" is only met visually; the drawing stops working.
- **Recommendation:**
  - Make `InstanceTree.dispatch` all-or-nothing: record `[node, instance, rendered]` before each
    mutation and restore them in a `catch` before rethrowing.
  - Alternatively, always redraw after a partial chain, and update D12 to say which one holds.
  - Add a two-level failure test; the current one, `instances.test.mjs:197`, is single-level.
- **Fix:** Chose all-or-nothing. `InstanceTree.atomically(work)` snapshots every node's descriptor, instance and rendered output (plus the node map and pass state) and restores them if `work` throws. `dispatch` runs inside it, and the hook wraps dispatch plus redraw in one transaction, so a redraw that throws is undone too. Added a two-level failure test and a draw-throws test. D12 is rewritten to match.
- **Verification:** `atomically` restores the node map, each node's descriptor, instance and rendered output, and the pass state. Instances are immutable, so putting the references back is a complete rollback. `dispatch` runs inside it, and the hook wraps dispatch plus redraw in an outer transaction. Both the two-level failure test and the draw-throws test pass, and D12 now states the all-or-nothing contract.

### ✅ Verified - RF3 DrawnUI callbacks look their token up in the live instance, so an event before React commits hits a stale or reused token
- **Severity:** Medium
- **Evidence:**
  - `sites/playground/src/render/DrawnTree.tsx:80-94` captures the node object, which is stable,
    and the token, which is tied to one generation.
  - After a dispatch, `source.instance` is replaced at once, but the new callbacks arrive only after
    the `ConcurrentRoot` reconciler commits.
  - Events that can fire again before that commit:
    - `ConsumeGestures`, which fires on every gesture (`SkiaControl.ts:1046`)
    - `Scrolled`, `TextChanged`
    - `StartChanged`/`EndChanged`
  - Re-initialization (`instances.ts:189`) resets the generation to 1, so a token such as `h1-2`
    can name a *different* handler after a parent redraw. A stale callback can then silently run
    the wrong handler.
- **Recommendation:**
  - In `bind`, capture `const instance = source.instance` and ignore the event when
    `source.instance !== instance`.
  - Or pass the captured instance to `dispatch` and look the handler up there.
- **Fix:** `bind` in `DrawnTree.tsx` captures `source.instance` when it draws, and the callback ignores the event (still returning `true`) when the node's instance has changed since. D12 records the guard.
- **Verification:** `bind` (`DrawnTree.tsx`) captures `source.instance` at draw time and drops the event when the node's instance has changed. Dispatch and redraw run synchronously inside the transaction, so any dispatch or re-initialization swaps the instance before the next event can arrive. D12 records the guard.

### ✅ Verified - RF4 Tapping the surviving drawing after a failed compile overwrites the compile diagnostics and revives cleared effects
- **Severity:** Medium
- **Evidence:**
  - The `result.ir === null` branch (`useNxDrawing.ts:87-97`) and the transport-error branch
    (`:71-80`) leave `session.current` on the old session.
  - The old dispatcher's `draw(current, …)` then rebuilds the whole state from that session
    (`:141-149`):
    - `diagnostics: current.diagnostics` (the old compile's, usually empty)
    - `effects: current.effects` (cleared in React state but still held by the session)
    - `compiling: false`
- **Recommendation:**
  - On a dispatch redraw, merge into the previous state:
    `setDrawing(prev => ({ ...prev, node, unknownControls, inertHandlers, effects, failure: null }))`.
  - Clear or retire the session's effects when a compile fails.
- **Fix:** A dispatch redraw now merges into the previous state (`node`, `unknownControls`, `inertHandlers`, `effects`) and keeps its diagnostics and `compiling`. The pipeline's own failure is kept in a ref and restored rather than cleared. Every failed-compile path (transport error, `ir === null`, prepare/draw throwing) goes through `failCompile`, which clears the surviving session's effects.
- **Verification:** A dispatch redraw now merges into the previous state and keeps its diagnostics and `compiling`; the failure comes from `pipelineFailure`. Every failed-compile path goes through `failCompile`, which clears the surviving session's effects.

### ✅ Verified - RF5 The handler node's component reference is resolved by name and can name a different component than the descriptor
- **Severity:** Medium
- **Evidence:**
  - `crates/nx-codegen/src/builder.rs:1719-1735` looks the handler's `component` name up in the
    binding module, then falls back to the emit's module.
  - D1 says the reference is built "the same way a descriptor's `component` reference is built, so
    a handler for an imported component resolves without name lookup."
  - Repro:
    - Source: `import { Button as ui.Button } from "../ui/button.nx"`, a local
      `external component <Button emits { Tapped { } } />`, and `<ui.Button onTapped=<Log /> />`.
    - The emitted handler names the *local* `Button` while the descriptor names `ui/button.nx:Button`.
    - The interpreter accepts it, since it compares names only.
    - The TypeScript runtime refuses it at `runtime/typescript/src/index.ts:2214` with the
      self-contradictory message "expected … Button.Tapped, got one for Button.Tapped".
  - The fallback's comment ("a handler … bound to an inherited emit") does not describe what the
    fallback actually covers.
- **Recommendation:**
  - Resolve the component from the enclosing descriptor's reference, or carry the component's
    module identity through HIR and resolve it with that.
  - Add an `ir_tests.rs` case for an aliased import beside a same-named local component.
  - Fix the comment.
- **Fix:** The builder no longer looks the handler's component up by name. `build_element_expression` resolves the element's tag once (the reference the descriptor uses) and passes it to a new `build_action_handler`. `build_expression`'s `ActionHandler` arm now reports that a handler outside an element property cannot be emitted; both HIR paths only place handlers there. Added `ir_tests::a_handler_names_the_component_its_descriptor_names`: an aliased import beside a same-named local now emits `handler ui/button.nx:Button.Tapped`. The misleading fallback comment is gone. Separately, the runtime's type-mismatch message now prints module keys when the two component names are equal.
- **Verification:** `build_element_expression` resolves the tag once and passes that same reference to `build_action_handler` (`builder.rs:1739-1862`), so the handler and the descriptor name the same component. HIR creates handlers only as element property values (`lower.rs:2029`, and the `components.rs:736` rewrite), and conditional fragments are rejected before their values are built, so the new diagnostic in `build_expression` is unreachable from a valid program. `a_handler_names_the_component_its_descriptor_names` would fail under the old name lookup. The runtime message now prints module keys when the two names are equal; that branch has no test (RF25).

### ✅ Verified - RF6 The `handlers` corpus never dispatches an action with fields and never dispatches against `Page`
- **Severity:** Medium
- **Evidence:**
  - Every invocation in `specs/ir-conformance/handlers/program.json` feeds
    `{ "$type": "Button.Tapped" }`, which has no fields. Rust and TypeScript are therefore never
    compared on:
    - the `action` slot (`env[actionSlot] = action`)
    - host-input normalization of an action with fields
    - a body reading `action.searchString`
  - The `Page` lifecycle (`program.json:49-52`) runs only an empty batch.
  - `Page` never renders `count` or `query` (`main.nx:40-47`), so a `Page.Update` would be
    invisible anyway.
  - D8's "a `Page` whose content children carry its handlers" is thus pinned only for token
    numbering.
- **Recommendation:**
  - Render `count` and `query` in `Page`.
  - Dispatch, in batches:
    - `h1-1` (a patch)
    - `h1-2` with a `SearchSubmitted { searchString }` action (a `DoSearch` effect carrying the
      string)
    - `h1-3` (a `query` patch from `action`)
- **Fix:** `Page` now renders `<Label text={count} />` and `<TextInput value={query} />`. Its lifecycle dispatches `h1-1` (count patch), `h2-2` with `SearchSubmitted { searchString: "docs" }` (a `DoSearch` effect carrying the string), `h3-3` with `"guides"` (a `query` patch from `action`), then an empty batch. The corpus was regenerated; the runtime corpus test passes. Runtime tests that index `Page`'s children now select them by `$type`.
- **Verification:** `Page` renders `count` and `query`. Its lifecycle dispatches `h1-1` (count becomes 1), `h2-2` with `searchString: "docs"` (a `DoSearch { search: "docs" }` effect), `h3-3` with `"guides"` (query becomes `"guides"`), then an empty batch. Both runtimes agree.

### ✅ Verified - RF7 The token-order and live-state corpus scenarios cannot tell a right implementation from common wrong ones
- **Severity:** Medium
- **Evidence:**
  - **Token order:**
    - `Card` declares `count header items` (`main.nx:7`), which is already alphabetical.
    - A runtime that numbers tokens in declaration or insertion order produces the same tokens, yet
      D4 names sorted order as what makes tokens agree across runtimes.
  - **Live state:**
    - In `Tens` batch 2, `h2-1` (the header, which reads state live) runs first, when live `count`
      (11) equals its captured value (11).
    - Only the loop's shadowing is actually distinguished.
- **Recommendation:**
  - Declare `Card`'s props out of alphabetical order (e.g. `title:Node` before `items:Node[]`).
  - Swap batch 2 to `h2-3` then `h2-1`, so the expected `count` is 121. That rules out
    captured-only (111) and shadow-overwritten outcomes.
- **Fix:** `Card` now declares `items header count`, and `Tens` writes `items` before `header`, so declaration or insertion order would number the loop first. Batch 2 is now `h2-3` then `h2-1`, and the recorded count is 121 (captured-only would give 111). Both runtimes agree.
- **Verification:** `Card` declares `items header count` and `Tens` writes `items` first, so sorted order (header `h-1`, then items) differs from declaration and insertion order. Batch 2 runs `h2-3` then `h2-1`, and `results.json` records `count` 121.

### ✅ Verified - RF8 The catalog generator's guard against a dropped parameter preceding a kept one can never fire
- **Severity:** Medium
- **Evidence:**
  - `sites/playground/scripts/generate-catalog.mjs:196-201` pushes to `fields` and `params`
    together, so `params.length !== fields.length` is always false.
  - The renderer maps `params[i]` to `args[i]` by position (`DrawnTree.tsx:90-92`).
  - A future `(sender, e: Args, value: boolean)` would record `["value"]` and fill `value` from `e`
    without any error. No current typing hits this.
- **Recommendation:**
  - Either throw when a parameter is kept after one was dropped,
  - or record dropped positions in `params` (e.g. `null`) and skip them in `bind`.
- **Fix:** `readEvent` sets a `dropped` flag and throws when a later parameter is kept. It explains why: the renderer fills fields by position. Regenerated catalog is byte-identical.
- **Verification:** `generate-catalog.mjs:189-201` sets `dropped` and throws when a later parameter is kept, explaining that the renderer fills fields by position. The regenerated catalog is byte-identical.

### ✅ Verified - RF9 The playground test suite does not cover the renderer's session behavior or the catalog's emits
- **Severity:** Low
- **Evidence:**
  - Nothing tests `useNxDrawing` or `drawRoot`. These spec scenarios rest only on the Playwright
    pass recorded in task 8.3:
    - "Editing resets the instances"
    - "Effects are cleared on recompile"
    - "A failed dispatch leaves the drawing"
  - No committed test binds `onNope`, reads `onToggled`'s `action.value`, or binds an inherited
    emit against the catalog; task 7.1 verified these by hand.
  - The unknown-emit diagnostic says "Element 'SkiaButton' has no property 'onNope'", while the
    catalog spec asks for "a diagnostic naming the unknown emit".
- **Recommendation:**
  - Add those catalog cases to `src/compile/catalog.test.mjs`.
  - Either reword the scenario to "naming the unknown property" or make the checker's message
    mention emits.
  - Consider factoring the session logic out of the hook so it can be tested without React.
- **Fix:** Added `catalog.test.mjs` cases: an inherited `onTapped` compiles; `onToggled` reads `action.value` as a boolean (and a string target is refused); `onNope` fails with a diagnostic naming the property. The catalog spec scenario and task 7.1 now say "naming the unknown property". The session logic moved into `InstanceTree.atomically`, which is tested; the React hook itself still has no test.
- **Verification:** `catalog.test.mjs:106-137` covers an inherited `onTapped`, `onToggled`'s boolean `action.value`, a refused string target (failing for the right reason), and `onNope`. The spec and task 7.1 now say "naming the unknown property". The session logic is tested through `InstanceTree.atomically`; the React hook itself still has no test, which the fix note acknowledges.

### 🔴 Open - RF10 The number-to-string gap is filed under capabilities NX now has
- **Severity:** Low
- **Evidence:**
  - These notes say readouts are blocked because NX cannot turn a number into text:
    - `animations.nx:7-9`
    - `looks.nx:7-8`
    - `scroll.nx:10-11`
    - `transforms.nx:6-7,72-74`
  - Those examples are tagged only `event-handlers`/`component-state`, so the gallery says "this
    port does not use event handlers or component state yet".
  - That wording presents a language gap as porting work, and the capability survey cannot find
    it.
  - `shell.nx:5-7` (shell methods) and `root.nx:5-6` (navigation) have the same shape.
- **Recommendation:**
  - Add a "missing" capability such as `number-formatting` and tag these examples.
  - Or state the limitation explicitly in the README and design D13.
- **Status:** Not changed. Adding a `number-formatting` capability changes the shared vocabulary, `check-examples`' list, the spec's wording and the gallery, which is a product call. The README already states the gap ("Most of the remaining readouts … wait on one language gap"). Recommendation: decide whether the gallery should name it; if so, add a `missing` capability and tag animations, looks, scroll and transforms.

### 🔴 Open - RF11 Some changed hunks belong to the binary/linkable IR change, not this one
- **Severity:** Low
- **Evidence:**
  - `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md:138` and `docs/drawn-ui-proposal-review.md:63,67`
    only restate that NX IR became a binary image. They are the follow-ups recorded in
    `openspec/changes/archive/2026-09-16-binary-nx-ir/review.md`.
  - `docs/nx-ir-format.md` has three unrelated hunks:
    - the workspace-version bullet (~l.244)
    - the damaged-image paragraph (~l.299-307)
    - the "Every SDK takes a module's version" paragraph (l.541-544), which also has a line over
      the wrap width
- **Recommendation:** commit these separately as the binary-nx-ir review follow-up.
- **Status:** Not changed. This is about how the work is split into commits, and nothing is committed yet. When committing, stage the three `docs/nx-ir-format.md` hunks and the two Drawn UI doc hunks as a separate binary-nx-ir follow-up commit, and rewrap line 541–544 then.

### ✅ Verified - RF12 FFI and .NET no longer test the IR codegen error path
- **Severity:** Low
- **Evidence:**
  - `ffi_codegen_nx_ir_returns_json_diagnostics_for_ir_errors` (`crates/nx-ffi/tests/ffi_smoke.rs:749`)
    and `GenerateNxIr_WithIrDiagnostics_ThrowsEvaluationException` (`NxEndToEndTests.cs:318`)
    were both repurposed to expect an image.
  - Nothing in those bindings exercises an IR codegen failure any more.
  - `sdk-node.test.ts` kept a failure by switching to a conditional property fragment.
- **Recommendation:** keep one failure case in each binding using that same fragment, beside the
  new success case.
- **Fix:** Restored `ffi_codegen_nx_ir_returns_json_diagnostics_for_ir_errors` and `GenerateNxIr_WithIrDiagnostics_ThrowsEvaluationException`, using the conditional property fragment and asserting `codegen-unsupported-construct`, beside the new success cases. `cargo test -p nx-ffi --test ffi_smoke`: 40 passed. The .NET `GenerateNxIr` filter: 6 passed, after rebuilding the release `libnx_ffi.so`.
- **Verification:** `ffi_smoke.rs:748` and `NxEndToEndTests.cs:318` keep the failure case (conditional fragment, `codegen-unsupported-construct`) beside the new success cases. FFI: 40 passed. .NET `GenerateNxIr`: 6 passed.

### ✅ Verified - RF13 Cross-module handlers and several D1 claims are tested only in Rust or not at all
- **Severity:** Low
- **Evidence:**
  - **Corpus:** no program has a handler or emit that crosses modules, so the TypeScript runtime
    never resolves an inherited emit's action from another module.
  - **`ir_tests.rs`:**
    - Nothing asserts `update-intrinsics-v1` is listed for an intrinsic inside a handler body,
      which D1 calls out.
    - `an_inherited_emit_references_its_declaring_module` checks only the `emits` block, not a
      handler bound to that emit.
    - The doc comment on `a_handler_emits_an_image_that_reads_back` says it uses the source of
      `component_action_handler_bindings_fail_before_emission`, but the sources differ.
- **Recommendation:**
  - Add an imported component with an inherited emit and a bound handler to `two-module` (or a
    second `handlers` module).
  - Add the two `ir_tests` assertions and fix the comment.
- **Fix:** `two-module` now has an abstract `Control emits { Tapped { } }` in `shared/model.nx` and a `Panel` in `app/main.nx` that binds `onTapped` on an imported-base `Tap`, with a lifecycle. Both runtimes resolve `Control.Tapped` across modules. Added `ir_tests` for a handler bound to an inherited emit (`action@0:ui/base.nx:Base.Tapped`) and for `update-intrinsics-v1` listed from inside a handler body. Reworded the `a_handler_emits_an_image_that_reads_back` doc comment.
- **Verification:** `two-module` adds `Control emits { Tapped { } }` in `shared/model.nx` and a `Panel` that binds `onTapped` on `Tap extends Control`. Its lifecycle dispatches the handler, and both runtimes pass (`Panel lifecycle`, with and without debug). The explained text shows `action@1:shared/model.nx:Control.Tapped owner Panel`. The inherited-emit handler and intrinsic-in-handler `ir_tests` exist, and the doc comment is fixed.

### ✅ Verified - RF14 The runtime re-resolves the handler owner through a hard-coded slot and a string split
- **Severity:** Low
- **Evidence:**
  - `evalActionHandler` (`runtime/typescript/src/index.ts:2025-2043`) resolves the owner's
    declaration and then keeps only a `module::Name` string.
  - `invokeHandler` (`:2350`) resolves the owner again with
    `resolveReference(handler.linked, 0, handler.owner.split("::").pop()!)`. That relies on slot 0
    being the module itself and on the key format.
  - `invokeHandler` is only ever given `liveState` when the owner is the dispatched component, whose
    declaration the caller already holds.
  - `splitHandlerProperties` takes an unused `program` parameter silenced with `void program;`
    (`:2216`).
  - `label` is built with `split("::").pop()` in several places.
- **Recommendation:**
  - Keep the resolved owner (and component) declarations on `ActionHandlerValue` beside the keys,
    or pass the dispatched component's kind into `invokeHandler`.
  - Drop the unused parameter.
- **Fix:** `ActionHandlerValue` carries `componentName`; labels no longer split keys. `invokeHandler` takes `live: { component, state }` from the dispatch, which already holds the component kind, so the slot-0 re-resolution is gone. Dropped `splitHandlerProperties`' unused `program` parameter.
- **Verification:** `ActionHandlerValue` carries `componentName`. `invokeHandler` takes `live: { component, state }` from dispatch, so the slot-0 re-resolution and the key splitting are gone. `splitHandlerProperties` no longer takes `program`.

### ✅ Verified - RF15 When a new compile's first draw throws, the surviving drawing silently ignores taps
- **Severity:** Low
- **Evidence:**
  - `useNxDrawing.ts:107` assigns `session.current = current` before `draw()` runs (`:108`).
  - The old drawing's dispatcher then returns early at `:155` because its session is no longer
    current.
- **Recommendation:** assign `session.current` only after the first draw succeeds, or show that the
  kept drawing is inert.
- **Fix:** `session.current` is assigned only after the first draw succeeds, so a failed first draw leaves the previous drawing dispatching through its own session.
- **Verification:** `session.current` is assigned only after `draw` succeeds (`useNxDrawing.ts`). A failed first draw leaves the previous session current, so its drawing keeps dispatching.

### ✅ Verified - RF16 The example check does not walk the tree the way the renderer does
- **Severity:** Low
- **Evidence:**
  - `scripts/expand-components.mjs:40-44` walks every field but reports only handlers stripped from
    authored descriptors.
  - `drawValue` walks only content, and also reports token-less handlers on plain controls
    (`DrawnTree.tsx:84`).
  - So `let root() = { <SkiaButton onTapped=<X/> /> }` warns on the site but passes
    `check-examples`, contrary to the spec's "expand … the way the renderer does".
  - `check-examples.mjs:70-73` prints `ok -` for an example that just recorded inert-handler
    failures, although the script still exits 1.
- **Recommendation:**
  - Report token-less handlers on plain controls in the walk.
  - Either limit the walk to the content property or document the difference.
  - Print `not ok` when failures were added.
- **Fix:** `expand-components.mjs` now walks like `drawValue`. For a catalog control it walks only the content property and reports token-less handler props as `<Type>.<prop>`. It expands authored components and does not look inside other values. Added a test for a root-level `<SkiaButton onTapped=... />`. `check-examples.mjs` prints `not ok - <name>` when the example added failures.
- **Verification:** `expand-components.mjs:30-59` now matches `drawValue`: arrays at any level, content only for catalog controls, authored descriptors expanded under their parent, and token-less handlers on controls reported as `<Type>.<prop>`. The new tests pass. `check-examples.mjs` prints `not ok` when failures were added during the example's pass; early-`continue` paths still print no status line (RF26).

### ✅ Verified - RF17 Inert-handler reports use two formats, one without the component name
- **Severity:** Low
- **Evidence:**
  - `bind` reports `SkiaButton.onTapped`, the format `useNxDrawing.ts:23` documents.
  - Stripped descriptor handlers are reported as `Children[0].onTapped` (`instances.ts:97`,
    test `:223`), so the pane reads "'Children[0].onTapped' is bound outside a component".
- **Recommendation:** prefix the path with the descriptor's `$type`, or use the control-type form
  `bind` uses.
- **Fix:** `stripInertHandlers` now reports `<Type>.<property>` after the value that carried the handler (`SkiaButton.onTapped`), the same form `bind` uses. Tests are updated.
- **Verification:** `instances.ts:95-99` reports `<$type>.<name>`, matching `bind`; the tests are updated.

### ✅ Verified - RF18 The README and playground spec describe a host effect the site cannot produce
- **Severity:** Low
- **Evidence:**
  - `sites/playground/README.md` and specs/playground/spec.md ("Effects the tree does not
    handle") both list "an action … returned by a root-level handler" as a host effect.
  - D11, and the README's next paragraph, say root-level handlers never run.
- **Recommendation:** reword both to "an action outside the component's contract returned by a
  handler, such as `<DoSearch />` from a page component".
- **Fix:** The README and the playground spec now say "an action outside the component's contract that a handler returns, such as `<DoSearch />` from a page component".
- **Verification:** `README.md:139-142` and `specs/playground/spec.md:58-61` use the new wording. The spec sentence has a wrapping and wording nit (RF27).

### ✅ Verified - RF19 Catalog generator edge cases diverge from the spec's event rule
- **Severity:** Low
- **Evidence:**
  - `eventSignature` (`generate-catalog.mjs:158-166`) never checks that the member is optional,
    which the spec's definition of an event requires. Every current event is optional.
  - When a subclass override folds away an inherited event (`:481-491`), the dropped-parameter
    entries `readEvent` already wrote to `omitted.json` for that subclass are kept.
    `overrides.json` has no event entries today.
- **Recommendation:**
  - Check `ts.SymbolFlags.Optional`.
  - Remove the matching omission entries when an event is folded.
- **Fix:** Only optional members are treated as events (`ts.SymbolFlags.Optional`). When an override folds an inherited event, that class's dropped-parameter omissions are deleted. Regenerated catalog is byte-identical.
- **Verification:** The fold cleanup is correct: `generate-catalog.mjs:493-499` deletes only that class's dropped-parameter omissions for the folded event. The optionality check at `:455-457` is not effective, though: every props symbol is Optional because `PropsOf<T>` wraps the props in `Partial<>` (`src/drawnui/react/index.tsx:46`). An instrumented run found no non-optional member, so the check always passes and the comment claims a rule the code does not enforce. Check the declaration's `questionToken` (or the original class member's `SymbolFlags.Optional`) instead, or document that `PropsOf` is where the filter lives. Output is unaffected today.
- **Fix:** (second pass) The optionality check now reads the DrawnUI class member's own `questionToken`, since `PropsOf<T>` makes every props symbol optional; the comment says so. Inverting the check once dropped the catalog to 0 emits, which shows it filters. With the correct check the regenerated catalog is byte-identical (43 emits).
- **Verification:** The check now reads `declaration.questionToken` (`generate-catalog.mjs:455`). `property.declarations[0]` is the DrawnUI class member itself, because the `Partial<>` mapped type keeps the original declarations, so the check really filters (the fix note records that inverting it dropped the catalog to 0 emits). The comment explains why the symbol flag cannot be used. A scratch regeneration printed 43 emits and was byte-identical to the committed catalog. The fold cleanup was verified in the previous pass.

### ✅ Verified - RF20 Reduced-coverage wording repeats itself and can end in a stray period
- **Severity:** Low
- **Evidence:**
  - The `reduced` branch at `sites/playground/src/examples/types.ts:79` produces text such as
    "That needs list virtualization. NX has no list virtualization yet." (cells, uneven-cells),
    and a doubled list for images, layouts, shaders and reorder.
  - `describeGaps` (`:94-104`) returns "." for an empty list. `check-examples.mjs:48` prevents that
    case today.
  - `types.ts:18-19` is missing a blank line between paragraphs.
- **Recommendation:**
  - Drop the "That needs …" sentence, since `describeGaps` already names each capability.
  - Return an empty string for no capabilities.
- **Fix:** Dropped the "That needs …" sentence. `describeGaps` returns `""` for no capabilities, and `listOf` lost its now-unused conjunction parameter. Added the missing blank line in the `Capability` doc comment.
- **Verification:** The "That needs …" sentence is gone, `describeGaps` returns `""`, and `listOf` has no conjunction parameter. `coverageNote` was evaluated for all 20 examples and every note reads cleanly. The blank line is at `types.ts:18`.

### ✅ Verified - RF21 Ported and reworded example sources have formatting and wording slips
- **Severity:** Low
- **Evidence:**
  - Unwrapped comment lines: `root.nx:6` (174 characters) and `snapping.nx:8` (~115).
  - The `component <Page /> = { … }` bodies in `text.nx:40-178` and `shapes.nx:37-238` are not
    indented. Visitors read these in the editor, and the README's page-component example is
    indented.
  - `text.nx:6` says the readouts are "reported below its card", but they sit inside the card.
- **Recommendation:** re-wrap the comments, indent the page bodies, and fix the comment.
- **Fix:** Rewrapped `root.nx:6` and `snapping.nx:8`. Indented the `Page` bodies of `text.nx` and `shapes.nx`, leaving continuation lines of multi-line strings untouched. `text.nx` now says the readouts sit "at the foot of its card".
- **Verification:** The comment lines in `root.nx` and `snapping.nx` are under 100 characters. The `Page` bodies in `text.nx` and `shapes.nx` are indented, the multi-line string literals are unchanged from HEAD (no indentation leaked into rendered text), and the `text.nx` comment is accurate.

### ✅ Verified - RF22 The explained text and damaged-image tests give node 19 little review signal
- **Severity:** Low
- **Evidence:**
  - `ir_explain.rs:600` prints `handler X.E action:A [owner O] =>` with no `action@<slot>`, and slot
    nodes print only their names. A slot or shadowing regression therefore reads the same in a
    corpus diff.
  - The Rust damaged-image suite (`ir_image.rs:1612`) damages only the smallest images. The
    `handlers` image is not among them, so node-19 cells and a non-empty `emits` list are never
    overwritten.
- **Recommendation:**
  - Print `action@N`, which task 1.3 suggested.
  - Also damage the smallest image containing an `actionHandler` node.
- **Fix:** The explainer prints `handler X.E action@<slot>:A [owner O] =>`. `ir_tests`, the FFI and .NET assertions, and the corpus text are updated. `every_cell_can_be_damaged_without_a_panic` also damages the smallest image that contains an `actionHandler` node.
- **Verification:** `ir_explain.rs:600-603` prints `action@<slot>`. `ir_image.rs:1616-1640` also damages the smallest image containing an `actionHandler` node, and that image has a non-empty `emits` list.

### ✅ Verified - RF23 Small dead code in the emitter and the corpus tests
- **Severity:** Low
- **Evidence:**
  - `crates/nx-codegen/src/ir_corpus_tests.rs:316-320`: `canonical_json` is now a wrapper that
    only calls `canonical_json_value`, and the doc comment sits on the wrapper.
  - `emit.rs:3063` and `:4084`: the `ActionHandler` arm's `nxRuntimeError("action handler")` output
    and helper collection are unreachable, because `validate_source_codegen_program` rejects
    handlers first. The observable behavior is correct.
- **Recommendation:**
  - Fold the wrapper into one function.
  - Make the emitter arm `unreachable!()` or drop the helper registration.
- **Fix:** Folded `canonical_json_value` into `canonical_json`. Both emitter arms for `ActionHandler` are now `unreachable!`, since both public entry points validate first.
- **Verification:** `canonical_json` is a single function (`ir_corpus_tests.rs:313-331`). Both emitter arms are `unreachable!` (`emit.rs:3062`, `:4088`), which is safe: the functions containing them are private, and both public entry points run `validate_source_codegen_program` first, whose match has no wildcard.

### ✅ Verified - RF24 tasks.md describes scenarios and sites the design ruled out
- **Severity:** Low
- **Evidence:**
  - Task 2.1 promises "a shadowing `let` around a handler" and "one inside a nested record".
    D8 says NX has no `let` expression, and the program has no record literal holding a handler,
    only nested descriptors.
  - Task 3.3 lists "component-typed host input in `normalizeValue`" as a split site, which D5 says
    does not exist. No `normalizeValue` change was made.
- **Recommendation:** reword 2.1 and 3.3 to match D5 and D8, or add a record-literal handler case.
- **Fix:** Task 2.1 now names the `for` shadowing, the out-of-order descriptor prop and D8's reason. Task 3.3 lists the host-input sites D5 names and points to D5 for why component-typed host input is not one. The design's risk bullet and D1's last paragraph are aligned to match.
- **Verification:** Task 2.1 matches D8 and the corpus program, and task 3.3 matches D5. D1's closing paragraph and the risk bullet are consistent; their mentions of `let` slots are accurate for the IR's `Let` kind.

## New Findings Discovered During 2026-09-16 Verification

### ✅ Verified - RF25 The runtime's type-mismatch message for two same-named components has no test
- **Severity:** Low
- **Evidence:**
  - The RF5 fix added a branch at `runtime/typescript/src/index.ts:2222-2229`: when the expected and
    actual components share a name, the message prints module keys.
  - No test in `runtime/typescript/test` asserts a message such as `…got one for ui/button.nx:Button.Tapped`.
- **Recommendation:** add a `runtime.test.ts` case that binds a handler for one module's `Button`
  on a same-named component from another module, and assert the message prints module keys.
- **Fix:** Added `runtime.test.ts` "a handler for a same-named component of another module is refused naming both modules". It links `main.nx` against `ui/button.nx`, binds a handler for the local `Button` on the imported one, and asserts `nx-ir-type` with "…for ui/button.nx::Button.Tapped, got one for main.nx::Button.Tapped."
- **Verification:** `runtime.test.ts:869-892` links `main.nx` against `ui/button.nx`, binds a handler for the local `Button` on the imported one, and asserts `nx-ir-type` with the module-key message. It exercises the equal-name branch, and it passes.

### ✅ Verified - RF26 The example check prints no status line for examples that fail early
- **Severity:** Low
- **Evidence:**
  - `sites/playground/scripts/check-examples.mjs:79-82` prints `not ok - <name>` only when a failure
    reaches the end of the loop.
  - These failures `continue` without printing any status line:
    - a missing source (`:37-40`)
    - a compile failure (`:61-66`)
    - an evaluation throw (`:76-78`)
  - The script still exits 1, but the per-example listing skips those examples.
- **Recommendation:** print `not ok - <name>` before each early `continue`.
- **Fix:** `check-examples.mjs` prints `not ok - <name>` before each early `continue` (missing source, compile failure, evaluation throw).
- **Verification:** `check-examples.mjs` now prints `not ok - <name>` before each early `continue`: missing source (`:39`), compile failure (`:65`) and evaluation throw (`:77`), plus the end-of-loop case. `pnpm test` shows 79 passed, 0 failed, and 20 examples checked.

### ✅ Verified - RF27 The reworded host-effect sentence in the playground spec is wrapped early and repeats itself
- **Severity:** Low
- **Evidence:**
  - `openspec/changes/ir-action-handlers/specs/playground/spec.md:60` breaks the line early, after
    "…page component — SHALL be shown to".
  - The sentence says "a handler returns" twice.
- **Recommendation:** rewrap the requirement paragraph and drop the repeated clause.
- **Fix:** Rewrapped the requirement paragraph and dropped the repeated "that a handler returns".
- **Verification:** The requirement paragraph in `specs/playground/spec.md` is rewrapped to the file's width, and "a handler returns" now appears once.

## Questions
- Should `ConsumeGestures` stay a catalog emit?
  - Binding it dispatches and redraws on every pointer event.
  - The renderer never sets `Consumed`, so it cannot do what the DrawnUI event is for.
  - It is also the event most exposed to RF3.

## Summary
- **Overall:** the compiler, image format and TypeScript runtime halves are solid.
  - Layouts agree across both readers and the explainer.
  - Slot allocation cannot collide.
  - Required features are listed only when needed.
  - Dispatch mirrors the interpreter's routing, live-state and atomicity rules.
  - All Rust, .NET, runtime and playground suites pass.
- **Main risk: the playground's instance tree.**
  - Multi-emit routing to one parent fails (RF1).
  - Mid-chain failures and pre-commit events leave callbacks holding stale or reused tokens
    (RF2, RF3).
  - The hook can overwrite compile diagnostics (RF4).
- **Compiler:** the handler's component reference is resolved by name, contrary to D1 (RF5).
- **Corpus:** the parity scenarios are weaker than they look, since no action has fields, `Page` is
  never dispatched, and the token order is indistinguishable (RF6, RF7).
- **Remaining findings:** test gaps, scoping and cleanup.
