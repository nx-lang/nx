# Review: add-environment-values

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/environment-values, specs/component-instance-tree,
specs/nx-ir-format, specs/rust-ir-runtime, specs/typescript-ir-runtime, specs/executable-code-generation,
specs/editor-language-service, specs/editor-syntax-highlighting, `specs/future.md` (environment section)  
**Reviewed code:** `git diff origin/main...HEAD`, focused on `crates/nx-ir-runtime/src/{environment,tree,component,eval,program}.rs`,
`runtime/typescript/src/index.ts` (environment section), `crates/nx-codegen/src/{runtime/environment.js,emit.rs,environment_cycles.rs}`,
`crates/nx-program/src/artifacts.rs` (provide-outside-main-module), `crates/nx-hir/src/{scope.rs,lib.rs}`,
`crates/nx-syntax/grammar.js`, `crates/nx-cli/src/main.rs`, `crates/nx-language-service/src/environment.rs`, the tree and
type-check tests. Findings RF1 and RF2 were reproduced with throwaway tests, since removed. RF4 and RF5 were checked with
`nx_types::check_str` probes.

## Findings

### 🟡 Fixed - RF1 An override on an external (host) component stops at it, so authored components in its content never see it, and `$env` leaks into host output
- **Severity:** High
- **Evidence:** `stamp` treats every record whose `$type` is a declared component as a component descriptor and stops
  there (`crates/nx-ir-runtime/src/environment.rs:866-877`). `is_component_name` counts every `DeclarationKind::Component`,
  and external components are emitted as component declarations with an `EXTERNAL` flag
  (`crates/nx-ir-runtime/src/program.rs` `is_component_name`, `crates/nx-codegen/src/ir.rs:1295`). The TypeScript runtime
  (`runtime/typescript/src/index.ts:4629`, `stampEnvironment`) and generated code (`nxDeclareComponents` in
  `crates/nx-codegen/src/emit.rs`, `crates/nx-codegen/src/runtime/environment.js:319`) do the same. The instance tree
  builds a node's environment only from the parent node and the node's own descriptor (`tree.rs`, `visit`: `self.node(parent)?.1.environment.enter(descriptor)`),
  and external components are not tree nodes, so their `$env` is never entered. Reproduced:
  `component <Page /> = { <Stack env.mode="dark"><Reader /></Stack> }` with `external component <Stack …>`. Page's output is
  `Stack { $env: [mode=dark], Children: [Reader {}] }`, and the `Reader` node visited under `root` renders
  `Label { Text: "light" }`. This breaks the main spec scenario "An override on a component use"
  (`<Section env.colorMode=dark><Links /></Section>`) whenever `Section` is a host control, which is the common case in a
  catalog-based app. It also goes against the design ("The rendered output of an intrinsic or host element carries no
  `$env`") and the IR spec ("an element that is not a component descriptor SHALL never get one"). A host renderer that
  passes descriptor fields through as native props now gets a `$env` field.
- **Recommendation:** Treat only authored components (those with a body, not `is_external`) as stopping points in all three
  stamp implementations. Walk through an external descriptor's fields and content as through an intrinsic element, and
  never add `$env` to it. Add tree, TS runtime and generated JS tests for `<HostControl env.x=…><Authored /></HostControl>`.
  Add a corpus case for it too, since the current `markup()` case only uses an undeclared `<div>`.
- **Fix:** Only a component declared with a body counts as a component descriptor in all three stamps (`ProgramData::is_component_name` in Rust, the environment index's `componentNames` in TypeScript, `nxDeclareComponents` in generated code), so an external component's fields and content are walked like an intrinsic element's and it never gets `$env`. Added the corpus case `hosted` (`<Stack env.colorMode=dark><Badge /></Stack>`, checked by both runtimes and generated JS), the tree test `an_override_on_a_host_component_reaches_the_components_inside_it`, and the spec scenario "An override on an external component reaches the components inside it" with the descriptor definition in `nx-ir-format`. Answer to the question: no, an external component carries no `$env` for now; `$env` on host elements stays in future.md.

### 🟡 Fixed - RF2 A read whose default is evaluated under the body's own override of another value records a value the skip check can never reproduce, so the node renders again on every visit
- **Severity:** Medium
- **Evidence:** `environment_value` records `(key, beneath)` (`crates/nx-ir-runtime/src/environment.rs:503-511`). `beneath`
  leaves out only the body's own overrides of the same key. A default or entry-module provide that produces the base is
  still evaluated with the body's local layers for other keys in effect (`provided_or_default` evaluates in the current
  machine). `environment_reads_hold` then re-reads at the node's base environment with no local layers
  (`environment.rs:942-954`), gets a different value and reports a change. Reproduced:
  `environment loud: string = { env.mode + "!" }` and
  `component <Card> = { <Stack>{ provide env.mode="dark" { <Label Text={env.loud} /> } }</Stack> }`. Visiting the root
  twice with the same descriptor gives a new rendered output (`Arc::ptr_eq` false) and new tokens. This breaks "The same
  descriptor leaves the node alone" and "An equal descriptor from a parent initialized again keeps the child". It also
  causes spurious renders from `set_environment` whenever any other value the node read changes. This pattern is the one
  the design promotes for cascading (`provide env.colorMode=dark { env.color… }`).
- **Recommendation:** Record a read only when the value it found doesn't depend on the evaluation's own layers. The simplest
  way is to record, for a read that reaches a default or provide, only the reads that evaluation makes, as is done now,
  and not the outer key's computed value whenever a local layer (position >= `base`) of any key was on the stack. Another
  way is to evaluate the recorded value with the local layers masked. Either way, add the two-visit test above, plus a
  `set_environment` test where an unrelated value changes.
- **Fix:** A read that reaches a default or entry-module provide while the evaluation's own overrides are in effect is recorded without a value, and `take_environment_reads` reads it again in a machine of its own over the node's environment, which is the read the skip check makes, uncharged. Added `a_body_that_cascades_its_own_override_is_left_alone_by_an_equal_visit` (an equal visit and a `set_environment` of an unrelated root value both keep the node); it fails without the fix.

### 🟡 Fixed - RF3 `provide-outside-main-module` only checks workspace modules, so a top-level provide in an imported library module is silently ignored
- **Severity:** Medium
- **Evidence:** `add_provide_outside_main_module_diagnostics` (`crates/nx-program/src/artifacts.rs:2309-2351`) collects
  `imported` only from workspace modules' imports into workspace identities (`.filter(|target| workspace.contains(target))`),
  and reports only on `modules` (workspace artifacts). Library modules (`libraries`) are never checked, either when the app
  builds or when the library is built alone. Both runtimes apply only the entry module's `$provides`, so a library module
  `theme.nx` with `provide env.space = 8`, imported by the app or by another module of its library, compiles cleanly and
  does nothing. The spec says "a module that another module of the program imports SHALL NOT contain a top-level provide".
- **Recommendation:** Run the same check over library modules: those imported by any module of the program, and those
  imported within their library when a library artifact is built. Add a test with a library module that holds a
  top-level provide.
- **Fix:** Library builds now run the check over their modules: a library module that another of its modules imports, or that exports a declaration (what a program importing the library reaches), is refused; a module that exports nothing, such as a preview `app.nx`, keeps its provides. The spec, the reference page and the test `a_library_module_a_program_reaches_cannot_hold_a_top_level_provide` say so.

### 🟡 Fixed - RF4 `provide` at the start of an expression breaks existing calls to a function named `provide`
- **Severity:** Low
- **Evidence:** `provide_expression` is a choice at the start of `value_list_item_expression` and `elements_expression`
  (`crates/nx-syntax/grammar.js:640-643`, `877-880`). `let provide(a: int) = { a }` still parses, but
  `let y() = { provide(1) }` is now a `syntax-error`. Before this change it was valid, since `provide` was an ordinary
  identifier. The design says `provide` is a keyword at the start of an expression, but the spec's promise that
  `environment`/`provide` "in any other position SHALL keep the scope the same position gives any other name" only covers
  fields and properties. Task 2.6 searched the repository only for `env`.
- **Recommendation:** Either commit `provide` to being a keyword only when followed by `env.` (grammar precedence or a
  lookahead on `property_value` starting with `env`), or reject `provide` as a declaration name with a clear diagnostic
  and record it in the spec. Search examples and docs for a `provide` function or local.
- **Fix:** `provide` is now reserved like `env`: a declaration, parameter or binding named `provide` is `reserved-name` with a message saying why, and a record field named `provide` keeps working. Recorded in the environment-values spec and the reference page; tested in `provide_cannot_be_declared_as_a_name`. A search of the repository's `.nx` and docs found no `provide` function or local.

### 🟡 Fixed - RF5 The design's `let caption(...) = provide … { … }` form does not parse
- **Severity:** Low
- **Evidence:** `design.md:65` shows `let caption(when: date) = provide env.locale="fr-FR" { formatDate(when) }`.
  `rhs_expression` (`crates/nx-syntax/grammar.js:593-599`) admits only an element, a literal, a contextual name or a braced
  expression, so the line is a `syntax-error` ("Invalid function definition"). The reference page and style guide use the
  braced form, so only the design is wrong. Still, the line is the design's headline example.
- **Recommendation:** Either add `provide_expression` to `rhs_expression` (it is already brace-delimited), or change the
  design example to `= { provide … { … } }`.
- **Fix:** The design example now uses the braced form, `= { provide … { … } }`, as the reference page and style guide do.

### 🟡 Fixed - RF6 Generated code does not check `$env` entries that carry a field path, unlike both IR runtimes
- **Severity:** Low
- **Evidence:** `nxWithEnvironment` normalizes only whole-value entries
  (`crates/nx-codegen/src/runtime/environment.js:146`: `entry.path.length === 0 ? nxNormalizeValue(…) : entry.to`). Rust
  (`host_environment_value` with `environment_type_at`) and TypeScript (`hostEnvironmentValue`) resolve the path's field
  type and check the value, failing with `nx-ir-environment` or a boundary diagnostic. A host of generated code that
  passes `{ value: "…::color", path: ["accent"], to: 5 }` gets no error, so the "kept equal" copies diverge. A misspelled
  path field also goes undetected until a read fails with a different error.
- **Recommendation:** Emit the field schemas a path can reach (or a path-to-schema helper) and check path entries the way
  the runtimes do, or document the difference. Add a codegen parity test with a path entry.
- **Fix:** `nxWithEnvironment` resolves an entry's path through the record schemas (`nxEnvironmentSchemaAt`) and checks the value against the field's schema, failing with `nx-ir-environment` for a path field that names no record field, as the runtimes do. The codegen test checks a valid path entry, a wrongly typed one and an unknown field.

### 🟡 Fixed - RF7 `--env` parses the value without the declared type, so a string value that looks like JSON is refused
- **Severity:** Low
- **Evidence:** `environment_from_args` (`crates/nx-cli/src/main.rs:250-268`) tries JSON first and falls back to a string,
  without looking at the declared type. Task 4.4 and the design say the value is read "against the declared type".
  `--env version=1.0`, `--env locale=true` or `--env code=123` for a `string` value become a float, bool or int and then
  fail the boundary check. No CLI test covers `--env` at all (neither scenario of "The CLI takes root environment values").
- **Recommendation:** Resolve the name first. When the declared type is `string` (or a union whose cases match the word),
  take the text as is, and parse JSON otherwise. Add CLI tests for `--env colorMode=dark`, `--env colorMode=dim` (the
  failure names `colorMode` and its type) and an unknown name.
- **Fix:** The CLI reads the arguments first and builds the values once the program is analyzed: a value of an environment value declared `string` is its text as written, and anything else is JSON or a bare word. Added `env_arguments_give_root_values_read_against_their_declared_types` (a union case, `version=1.0` as a string, an int, a wrong case naming `colorMode`, an unknown name, and a malformed argument).

### 🟡 Fixed - RF8 Two doc comments were split from their items when new items were inserted
- **Severity:** Low
- **Evidence:** In `crates/nx-ir-runtime/src/component.rs:59-65`, `/// The name of the component the instance is of.` now
  sits above the new `reads()` method, so `reads()` has two summaries and `component()` (a public API) has none. In
  `crates/nx-ir-runtime/src/tree.rs:1326-1360`, the doc of `same` ("Whether two values are the same value…") now sits
  above `same_descriptor`, and `same` has none.
- **Recommendation:** Move each comment back to its item.
- **Fix:** Moved each comment back to `component()` and `same`.

### 🟡 Fixed - RF9 Tests are missing for the riskiest paths: externals, ambiguity, imports, bundles in the tree, and the CLI
- **Severity:** Medium
- **Evidence:** Task 8.1 is still unchecked, which is consistent with this. Spec scenarios with no test found:
  `ambiguous-environment-value` naming both modules; a read of an imported value; `env` rejected as a `for` item or pattern
  binding (only a parameter and a bare `env` are tested in `crates/nx-types/tests/environment_values.rs:83-87`); the CLI
  scenarios; an override around a host control (RF1); a body-local override that cascades through a default in the tree
  (RF2); a library module with a top-level provide (RF3); and source-level tests through both runtimes and generated JS
  for "An override's value reads the value outside it", "A later override applies on an earlier one" and "A bundle field
  updates part of a record". The corpus covers some of these for the runtimes, but not from source through generated JS.
- **Recommendation:** Finish 8.1 with these cases first, since RF1 to RF3 show they are where the defects are.
- **Fix:** Added tests for each listed gap: host components (tree test and corpus case through both runtimes and generated JS), the cascade (RF2), library provides (RF3), the CLI (RF7), `env` as a `for` item and a declaration, `provide` as a name, an aliased imported read and stacked overrides through generated JS, and an ambiguous read across two imports. The ambiguous-read test found a real gap: a second unaliased import of the same name was dropped silently, so `env.colorMode` resolved to the first import. Prepared modules now keep such environment bindings (`hide_environment_binding`), and the checker reports `ambiguous-environment-value` naming both modules.

## Questions
- For RF1: should an external component be able to carry `$env`, for example for a future "host elements read the
  environment" design (the design's Open Question)? If so, the tree still has to enter it for the authored components in
  its content, and the spec text "an element that is not a component descriptor SHALL never get one" needs to say which
  descriptors count.
- Is the bare-word or JSON ambiguity in RF7 acceptable as specified ("JSON, or a bare word")? The known deviation covers
  the format, but not the case of a string value that parses as JSON.
- **Answers (fix pass):** RF1: no, for now an external component never carries `$env`; the walk goes through it, and the
  spec now defines a component descriptor as a component declared with a body. RF7: no; a value declared `string` is now
  its text as written, and the spec, design and reference page say so.

## Summary
- The core lookup, precedence, field paths, capped provides and bundles are implemented consistently across the Rust
  runtime, the TypeScript runtime and generated JS, and the checks for declarations, reserved `env`, paths and duplicates
  behave as specified in the probes.
- There are two confirmed runtime defects. Overrides placed on host controls don't reach the authored components inside
  them, and `$env` leaks onto host output (RF1, High). Read tracking makes nodes re-render on every visit when a body
  cascades a local override through a default (RF2).
- The main-module provide check misses library modules (RF3). The other findings are small grammar, CLI and documentation
  issues and test coverage, which task 8.1 still has open.
