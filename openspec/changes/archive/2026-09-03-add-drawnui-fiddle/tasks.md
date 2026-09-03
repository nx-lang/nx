## 1. Fix list-typed content binding in the TypeScript IR runtime

- [x] 1.1 Make `applyContentBinding` in `runtime/typescript/src/index.ts` aware of the content
      field's declared type (unwrapping `nullable`), binding a list whenever that type is a list and
      preserving direct binding otherwise; verify at both call sites
      (`constructComponentDescriptor`, `evalComponentDescriptor`)
- [x] 1.2 Add runtime tests covering one child, several children, and a non-list content property,
      and verify `npm test` in `runtime/typescript` passes
- [x] 1.3 Verify parity with the Rust interpreter: compile a program binding a single child to a
      list-typed content property, and assert `nxlang run --format json` and the TypeScript runtime
      agree on that property's value
- [x] 1.4 Verify the existing runtime suite still passes, confirming no regression for non-list
      content properties
- [x] 1.5 Make `normalizeNominalValue` drop a record value's own `$type` discriminator before
      matching fields, as the union branch beside it already does, so a constructed record normalizes
      into a record-typed field; add runtime tests for an explicit record property, a record-valued
      default, and parity with the Rust interpreter
- [x] 1.6 Make `normalizeValue` treat a single value at a list-typed field as a list of one, the
      coercion the language already performs, so `Shadows={ <SkiaShadow /> }` evaluates; add runtime
      tests for a scalar, a record, a list of several, and parity with the Rust interpreter
- [x] 1.7 Check a record value's `$type` before dropping it, so host input naming another type is
      rejected with `nx-ir-boundary-type` rather than restamped with the declared name, while a
      plain object carrying no discriminator is still accepted; add runtime tests for both

## 1b. Type check component bodies in `crates/nx-types`

- [x] 1b.1 Infer each `Item::Component` in `check_file`'s item loop, adding `infer_component` to
      `InferenceContext`: bind props and state by name in a pushed scope, infer the body, and pop —
      so a prop cannot outlive the component that declared it; verify a bare contextual case in a
      component body resolves and that `nxlang codegen --target nx-ir` no longer reports
      `unresolved contextual name cannot be emitted`
- [x] 1b.2 Check each prop and state default against the field's own declared type under
      `component-default-type-mismatch`, and verify a bare case is accepted as a default while a
      quoted string at a union-typed prop is rejected with the message naming the cases
- [x] 1b.3 Verify a property type mismatch inside a component body is now reported, and that it
      reports the same diagnostic as the identical element at the top level
- [x] 1b.4 Add a `Type::Named` arm to `infer_member_access` resolving a record field through the
      record's effective shape, with each field's type resolved in the module that declared that
      field; verify a declared field, an inherited field, and a field whose type is a same-named
      union in another module
- [x] 1b.5 Report an unknown record field naming the fields the record has, rather than falling
      through to `Member access not yet implemented`
- [x] 1b.6 Verify the whole workspace test suite still passes, and that the one test it broke
      (`js_program_module_flattens_cross_module_values_types_unions_and_components`, which reads
      `user.name` in a component body) passes because of 1b.4 rather than by being changed
- [x] 1b.7 Rewrite the examples' qualified union forms to the bare form now that both work, drop
      the workaround note from `shapes.nx` and the qualified-form rule from `CATALOG.md`, and verify
      the emitted IR is unchanged apart from spans and slot numbering for all twelve examples
- [x] 1b.8 Unwrap a nullable base in `infer_member_access` so a nullable record or union prop reads
      its field, returning the field's own declared type; verify against a running fiddle that
      `Item:Contact?` with `Item.Title` in a component body compiles, and that an unknown field on a
      nullable base is still rejected
- [x] 1b.9 Bind the component's *effective* props rather than its declared ones, resolving each
      inherited field's type in the module that declared it, so a body reading a prop it inherits is
      checked instead of inferring vacuously; verify an inherited prop at a mismatched site reports
      what a declared one reports
- [x] 1b.10 Give a prop or state default one scope, the fields materialized before it, and use it in
      undefined-name checking, inference and code generation alike: check each default before
      binding its own field, walking the effective props then the state; verify a forward reference,
      a self reference and an unknown name are each reported, and that naming an inherited prop or
      an earlier field still works
- [x] 1b.11 Report a name that reaches neither a binding nor a declaration during code generation
      rather than emitting an `unresolved:` slot no runtime can bind, leaving the base of a dotted
      import alias — which is a spelling, not a value — emitted as before
- [x] 1b.12 Emit the effective field set for a record declaration, a record construction and a union
      case, each inherited field's default and type resolved in the module that declared it; verify
      against the interpreter that an inherited field, an inherited default and a union case's base
      fields all reach the TypeScript runtime
- [x] 1b.13 Resolve type aliases when building an IR type reference, with cycle protection, so a
      list spelled `type Ints = int[]` takes the single-value coercion and the list content binding;
      verify parity with the interpreter through single- and multi-hop nullable aliases
- [x] 1b.14 Emit each record's and union's base chain in the IR as declaration references, nearest
      first, and accept a value at a base-typed boundary whose `$type` names a declaration extending
      the expected one, normalizing it against its own schema and keeping its own discriminator;
      verify against the interpreter that a derived record and a union case both pass at a
      base-typed property, that a record not extending the base is still rejected, and that a name
      two declarations share is reported rather than guessed at
- [x] 1b.15 Emit `isAbstract` on each record declaration in the IR, and reject a value normalized as
      an abstract record — carrying no discriminator, or one naming that record or another abstract
      record extending it — while continuing to accept a concrete record that extends it; cover both
      host spellings with tests

## 1c. Allow an empty element body in `crates/nx-syntax`

- [x] 1c.1 Make body content optional in the `element` rule's open/close form in
      `crates/nx-syntax/grammar.js`, so an opening tag closed immediately by its own closing tag
      parses; regenerate `src/parser.c` and `src/grammar.json` with the pinned `tree-sitter` CLI and
      verify the generator's warnings are unchanged from the pre-change baseline
- [x] 1c.2 Verify an empty body means what a self-closing tag means — the declared content property
      is left unset — and that a whitespace- and comment-only body is empty
- [x] 1c.3 Verify an empty body is accepted wherever an element is written, both as a file's
      trailing element and inside `let root()`, since the defect was never specific to top level
- [x] 1c.4 Verify supplying content is unaffected: a target declaring no content property still
      accepts an empty body and still rejects a populated one, and a closing tag naming a different
      element is still reported as a mismatch
- [x] 1c.5 Add parser regression tests for the empty body, the whitespace-only body with properties,
      and the top-level empty element, and verify the whole workspace test suite passes

## 1d. Make the external scanner terminate in `crates/nx-syntax`

- [x] 1d.1 Replace the two `TSLexer` save-and-restore lookaheads in `crates/nx-syntax/src/scanner.c`
      — the `@{` opener in typed text content and the entity start — with decisions made by
      consuming, since restoring the struct rewinds the lookahead character but not the position it
      was read from, and at end of input the restored character comes back forever
- [x] 1d.2 Verify the parse trees are unchanged, by diffing `tree-sitter parse` output across text,
      typed-text, entity, escape, and raw-content cases before and after
- [x] 1d.3 Verify termination by fuzzing short delimiter-heavy inputs through the parser and
      confirming none exceeds a time limit, where the same corpus hangs the previous scanner
- [x] 1d.4 Add parser regression tests that parse on a worker thread with a deadline, so a scanner
      that fails to terminate fails the suite rather than hanging it, and verify the whole workspace
      test suite passes
- [x] 1d.5 Rebuild the native Node addon so the fiddle's compiler carries the fixed parser, and
      verify a source ending in a stray delimiter comes back as a diagnostic

## 2. Project scaffold and vendored DrawnUI

- [x] 2.1 Create `sample-apps/drawnui-react/` as a self-contained npm project (Vite, React 19,
      TypeScript) with `file:` dependencies on `runtime/typescript` and `bindings/node`, and verify
      `npm install && npm run typecheck` succeeds
- [x] 2.2 Add `scripts/sync-drawnui.mjs` copying `~/src/DrawnUi.React/src/` into `src/drawnui/` and
      its demo pages into `reference/demo-pages/`, writing the upstream commit hash and copy date to
      `src/drawnui/UPSTREAM.md`; verify a run produces both trees and the recorded revision
- [x] 2.3 Copy DrawnUI's shared font assets into `public/fonts/`, mirror the Vite `publicDir` and
      `build.target: "esnext"` settings from DrawnUI's `samples/vite.shared.ts`, and verify the
      CanvasKit wasm and font files resolve in a dev build
- [x] 2.4 Render a hardcoded DrawnUI tree (a `SkiaLayout` with two `SkiaLabel` children) through
      `Super.UseDrawnUi().ConfigureFonts(...).BuildAsync()` and `<Canvas>`, and verify it draws in
      the browser before any NX is involved

## 3. Catalog generator

- [x] 3.1 Write `scripts/generate-catalog.mjs` using the TypeScript compiler API to resolve
      `PropsOf<T>` for every tag in DrawnUI's `Registry`, and verify it reports the full control
      list including accessor-defined props such as `SkiaLabel.Text` and `SkiaLabel.FontSize`
- [x] 3.2 Implement the type mapping from design.md (primitives, literal unions, records, `Color`
      and `GridLength` as strings, function-typed props omitted), and verify the mapping table is
      exercised by at least one control per row
- [x] 3.3 Emit `catalog/skia.nx` with abstract external components for DrawnUI's shared bases,
      concrete components extending them, content properties typed as a list of the catalog's node
      root, and every property optional with no defaults mirrored; verify
      `nxlang codegen catalog/skia.nx --target nx-ir -o <tmp>` succeeds with no diagnostics
- [x] 3.4 Emit `catalog/catalog-meta.json` naming union types, record types, and which records need
      class construction, and verify every record type in `skia.nx` appears in it
- [x] 3.5 Verify generation is reproducible by running the generator twice and diffing both outputs
- [x] 3.6 Commit the generated `skia.nx` and `catalog-meta.json`, and verify a clean checkout can
      build without running the generator
- [x] 3.7 Record catalog divergences from DrawnUI (simplified types, omitted props, any edits made
      to the vendored copy) in `docs/CATALOG.md`, and verify each simplification named in the design
      appears there

## 4. Compile service

- [x] 4.1 Build `@nx-lang/sdk-node` locally (`npm run build` in `bindings/node`) and verify
      `generateNxIrFromSource` returns IR for a trivial NX program from this app's `node_modules`
- [x] 4.2 Implement `server/compile.mjs`: concatenate the catalog followed by the visitor's source,
      call `NxProgramArtifact.buildSource(...).generateNxIr()`, and return `{ ir, diagnostics }`;
      verify a program using catalog controls compiles and one with a typo returns a diagnostic
- [x] 4.3 Classify diagnostics by whether their line falls within the leading catalog or the
      visitor's source, returning catalog-positioned ones as application faults; verify with a
      program that triggers each case
- [x] 4.6 Shift a visitor-positioned span back into the visitor's own coordinates by the catalog's
      leading line and byte counts, leaving columns untouched; verify a file that is a single
      trailing element compiles, and that an error on a known line still reports that line and
      column after the shift
- [x] 4.4 Implement `server/index.mjs` serving the built SPA plus `POST /api/compile`, with a request
      body size limit; verify oversized and malformed requests are rejected without crashing
- [x] 4.5 Verify diagnostic line and column numbers match the visitor's source exactly, by
      submitting a program with a known error on a known line and asserting the reported position
- [x] 4.7 Treat a label as unpositioned only when it carries the whole-program sentinel — an empty
      span at the combined module's first character — rather than whenever it has no width, so an
      insertion point such as a missing `}` is reported against the visitor's source; widen an empty
      span by one column where the editor marks it, since a marker with no width draws nothing
- [x] 4.8 Guard the request boundary in `server/index.mjs` so no single request can end the process:
      answer a path the URL decoder rejects with 400, and catch anything escaping a handler; verify
      over HTTP that a malformed escape and a source the compiler chokes on are both answered, and
      that the service still serves the request that follows
- [x] 4.9 Assert liveness in `server/index.test.mjs` by the exchange completing rather than by the
      status, since the `503` a server with no `dist/` returns is a live answer; verify the suite
      passes with `dist/` moved aside, which is the state a fresh checkout runs it in

## 5. Client compile seam and evaluation

- [x] 5.1 Define the compile interface (`compile(source) => { ir, diagnostics }`) with an HTTP
      implementation, and verify the editor and renderer import only the interface
- [x] 5.2 Prepare and evaluate returned IR with `@nx-lang/ir-runtime`
      (`prepareNxIrProgram` + `evaluateFunction(program, "root")`), and verify a catalog program
      evaluates to a `$type`-tagged value tree
- [x] 5.3 Handle evaluation and transport failures without losing the session, and verify the app
      stays editable after a forced server error

## 6. IR-to-DrawnUI renderer

- [x] 6.1 Walk the evaluated tree mapping `$type` to a DrawnUI tag via `createElement`, and verify a
      single `SkiaLabel` draws
- [x] 6.2 Coerce property values using `catalog-meta.json`: unwrap union cases to their case name,
      construct `Thickness`/`CornerRadius`/`SkiaShadow`/`SkiaPoint`, pass other records as plain
      objects, and drop nulls so DrawnUI defaults survive; verify one case per coercion kind
- [x] 6.3 Normalize the content property to an array so a single child and several children both
      draw, and verify both against the runtime fix from group 1
- [x] 6.4 Render an unknown `$type` as a visible placeholder and report it, and verify the rest of
      the tree still draws
- [x] 6.5 Verify nesting end to end: a program nesting layouts, labels, shapes, and a scroll region
      draws correctly, and the scroll region scrolls

## 7. Fiddle view

- [x] 7.1 Mount Monaco in a two-pane fiddle layout with the canvas, and verify both panes render and
      resize
- [x] 7.2 Wire TextMate highlighting through `vscode-textmate` and `vscode-oniguruma` using
      `src/vscode/syntaxes/nx.tmLanguage.json` imported from the repository, and verify NX keywords,
      strings, and element syntax highlight
- [x] 7.3 Debounce edits and compile on pause rather than per keystroke, and verify a burst of typing
      produces one compile request
- [x] 7.4 Surface visitor-positioned diagnostics as Monaco markers, and verify a marker appears on
      the expected line with a readable message
- [x] 7.5 Keep the last successful drawing when compilation fails, and verify the canvas is unchanged
      after introducing a syntax error
- [x] 7.6 State in the UI that authored interaction is not yet supported, and verify the notice is
      visible without scrolling

## 8. Example ports

Port in the stated order, cheapest first, so the catalog and renderer are proven before the hard
pages. Each port is NX authored against the catalog, compiling with no diagnostics, with its coverage
state and capability tags recorded in example metadata.

- [x] 8.1 Define the capability vocabulary (`event-handlers`, `animation`, `component-state`,
      `list-virtualization`) and the example format — NX file plus metadata carrying name, coverage
      state (`complete`/`static`/`reduced`) and tags; verify one example loads, compiles, and draws
      through the app's own pipeline
- [x] 8.2 Port `Shapes` as `complete` and verify the drawn result matches the DrawnUI original side
      by side
- [x] 8.3 Port `Layouts` as `complete` and verify it matches the original
- [x] 8.4 Port `Images` as `complete` and verify it matches the original, with bundled image assets
      resolving
- [x] 8.5 Port `Svg` as `complete` and verify it matches the original
- [x] 8.6 Port `Text`, `Looks`, and `Root`, trimming interactive bits, and verify each draws and
      declares its state and tags
- [x] 8.7 Port the declarative substance of `Transforms`, `Snapping`, and `Accessibility` as
      `static`, dropping animation and handlers; verify each draws, names its capability tags, and
      carries an in-source note where the dropped behavior belonged
- [x] 8.8 Author `Snapping` (and any other interaction-driven port) to rest at a deliberate position,
      and verify no control is drawn mid-transition in a way that reads as a bug
- [x] 8.9 Port `Cells` and `UnevenCells` as `reduced` over a short fixed list, tagged
      `list-virtualization`, and verify each draws and states what the original demonstrates
- [x] 8.10 Verify every example compiles with no diagnostics, that every non-`complete` example names
      at least one capability tag, and that every tag used is in the vocabulary — as one check over
      the example set

## 9. Gallery

- [x] 9.1 Build the gallery view listing every example by the name the DrawnUI demo site gives it,
      and verify each entry draws its example from NX through the app's own pipeline
- [x] 9.2 Add a fiddle affordance per entry opening `/fiddle/<example>` with that example's NX
      loaded, and verify the fiddle draws the same example the gallery showed
- [x] 9.3 Add routing so a fiddle address loads directly without passing through the gallery, and a
      way back to the gallery; verify both directions
- [x] 9.4 Verify edits made in the fiddle do not alter the gallery entry on return
- [x] 9.5 Render coverage chips derived from each example's state and tags — nothing for `complete`,
      a quiet neutral chip for `static`, a stronger one for `reduced` naming what the original
      demonstrates; verify no chip is styled as an error and that two examples sharing a tag word it
      identically
- [x] 9.6 Carry the same coverage note into the fiddle view above the source, and verify it matches
      the gallery's wording for the same example
- [x] 9.7 Verify every gallery entry resolves to compiling NX and that its fiddle affordance opens
      it — no placeholder entries

## 10. Documentation and deployment

- [x] 10.1 Write `sample-apps/drawnui-react/README.md` covering prerequisites (Node, Rust toolchain),
      local run steps, the DrawnUI sync script, and the catalog generator; verify a fresh clone can
      be run by following it
- [x] 10.2 Document the static-only scope, the toolchain workarounds (NXE12/NXE13, braced union
      defaults, union serialization divergence), and any DrawnUI demo page with no NX example and
      why; verify each is described with its workaround or reason
- [x] 10.3 Add a Dockerfile building the native addon, the catalog, and the SPA into one Node
      service, and verify the image builds from a clean checkout and serves the app
- [x] 10.4 Verify the built image serves the gallery, the fiddle, and `POST /api/compile` from a
      single service, with no artifact required from outside the image
- [x] 10.5 State in the README's deployment section that `POST /api/compile` compiles synchronously
      on the server's only thread with no deadline, that only a child process could interrupt it,
      and what that means before pointing untrusted traffic at it
