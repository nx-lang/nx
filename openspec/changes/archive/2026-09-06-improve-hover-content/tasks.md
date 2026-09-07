## 1. Measure and close the inference hole

- [x] 1.1 Add failing checker tests in `crates/nx-types` for the `source-analysis-pipeline` delta's
  scenarios — a type error in an unresolved element's content, one in its property value, a resolved
  element nested inside an unresolved one, an unresolved tag reporting nothing beyond the tag, and a
  recorded type for an expression inside an unresolved element — and verify each fails today for the
  stated reason (`cargo test -p nx-types`)
- [x] 1.2 Make `infer_element_expression`'s unresolved fallthrough (`crates/nx-types/src/infer.rs:1459`)
  infer every expression in `element.content` and every property value before returning the nominal
  type for the tag, per design D1, and verify the 1.1 tests pass
- [x] 1.3 Record the full diagnostic delta this creates across the repo — run the checker over
  `examples/nx/`, `sample-apps/`, and the DrawnUI catalog before and after, and write the diff into
  the change directory as `measured-diagnostics.md`, classifying each new diagnostic as a source
  defect the checker was hiding or a checker defect the recursion exposed
- [x] 1.4 Review the 1.3 classification against design.md's Open Question, and if it shows correct
  programs being rejected because inference is weaker inside markup, stop and report before
  continuing to group 2

## 2. Repair what the checker was hiding

- [x] 2.1 Fix every source defect classified in 1.3 in `examples/nx/` that NX gives the author a way
  to fix, and record the rest. The four field reads off a fieldless `object` are fixed; the four
  `.length` reads on a list are left, because NX has no list length member to write instead —
  see `measured-diagnostics.md`. "Verify the examples check clean" is dropped: `complex.nx` carries
  six pre-existing syntax and resolution errors that predate this change and are not its to fix
- [x] 2.2 Fix every source defect classified in 1.3 in `sample-apps/` and the DrawnUI catalog, and
  verify they check clean
- [x] 2.3 Fix every checker defect classified in 1.3, each with a regression test in `crates/nx-types`,
  and verify `cargo test --workspace` is green

## 3. Position contexts for the unanswered positions

- [x] 3.1 Add `PositionContext::MemberAccess { member, span }` in
  `crates/nx-language-service/src/positions.rs` carrying the span of the whole member-access
  expression per design D3, claim the member identifier before `name_context` sees it, and verify
  with `positions` unit tests that `u.na⟨cursor⟩me` resolves to it while `u⟨cursor⟩.name` still
  resolves to the receiver
- [x] 3.2 Add `PositionContext::LocalDeclaration { kind, name, owner, span }` covering a function
  parameter, an element-style function parameter, a record field, a union case, and a union payload
  field, and verify with `positions` unit tests that each resolves to it rather than to `Reference`
- [x] 3.3 Give `PositionContext::PropertyValue` a `name: Option<String>` and the written name's real
  span per design D4, and verify with a `positions` unit test that `<Card role=ad⟨cursor⟩min />`
  reports `admin` with a non-empty span while `<Card role=⟨cursor⟩ />` still reports `None`

## 4. Hover answers at the new positions

- [x] 4.1 Add a lookup that resolves a visible `Declaration` to its HIR `Item` through
  `origin: (module identity, LocalDefinitionId)` and the artifact map per design D2, and verify with
  a unit test that it reaches a function's parameters, a record's fields, and a union's cases
- [x] 4.2 Handle `MemberAccess` in `hover_contents` — report the member and the access's type — and
  verify the "Hover over a member of a member access reports the member" scenario
- [x] 4.3 Handle `LocalDeclaration` in `hover_contents` for all five kinds, reading the written
  annotation where there is one per design D5, and verify the parameter, record-field, and
  union-case declaration scenarios
- [x] 4.4 Resolve a written bare `PropertyValue` through the same path completions use, and verify
  the "Hover over a bare property value reports what the name resolves to" scenario, including that
  it matches the answer at a typed `let` site
- [x] 4.5 Verify hover now answers inside an unresolved-tag element, covering the
  "Hover answers inside an element whose tag resolves to nothing" scenario, which group 1 enabled

## 5. NX-fenced hover rendering

- [x] 5.1 Add NX spelling helpers that render a declaration as an author writes it —
  `let add(count:int): int`, `let <Panel title:string />`, `type Size = int`, `let num: UserId` —
  with unit tests over each declaration kind, leaving `Declaration::detail` untouched per design D6
- [x] 5.2 Replace `declaration_hover`, `property_hover`, and `builtin_type_hover` with a renderer
  that emits a ```` ```nx ```` fenced block, states the kind once, and uses the parenthesized-kind
  prefix per design D7, and verify the four "Hover content is markdown written in NX" scenarios
- [x] 5.3 Update every existing hover assertion in `crates/nx-language-service/src/lib.rs` to the
  fenced content, and verify `cargo test -p nx-language-service` is green

## 6. Richer content for declarations

- [x] 6.1 Report a union's full case list — read from HIR, not from the payloadless-filtered
  `Declaration::members` — and verify the "Hover over a union declaration reports its cases" scenario
- [x] 6.2 Report a record's fields and their declared types, and verify the
  "Hover over a record type declaration reports its fields" scenario
- [x] 6.3 Report the inferred type of an unannotated value declaration from `type_env` per design D5,
  and verify the "Hover over an unannotated value declaration reports its inferred type" scenario

## 7. Verify end to end

- [x] 7.1 Confirm the conservative contract still holds: re-run the existing
  `hover_returns_none_for_unknown_syntax`, `hover_on_a_keyword_returns_no_result`,
  `hover_over_an_empty_property_slot_returns_no_result`,
  `hover_over_an_empty_type_annotation_returns_no_result`, and
  `hover_inside_a_declaration_with_a_syntax_error_returns_no_result` tests unchanged, and verify each
  still passes
- [x] 7.2 Update the `nx-lsp` delegation tests in `crates/nx-lsp/src/lib.rs` for the new markdown
  content and verify `cargo test -p nx-lsp` is green, with no change to advertised capabilities
- [x] 7.3 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`.
  `cargo test --workspace --no-fail-fast` is green: 52 test binaries, no failures. (Earlier in this
  change `nx-codegen`'s two `generated_*` tests failed on a TypeScript error in the generated
  `nx-runtime.ts`; they failed with the change stashed too, and they now pass — the cause was
  outside this change either way.) `cargo clippy --workspace --all-targets` is *not* clean at the
  repo baseline and is not made worse here: it fails on `approximate value of f{32,64}::consts::PI`
  in `nx-hir` and `nx-value` lib tests plus an unresolved `criterion` import in the `nx-syntax`
  bench, all in crates this change does not touch. What was verified clean is `cargo clippy -p
  nx-language-service -p nx-lsp -p nx-types -p nx-api -p nx-codegen --all-targets`, which reports no
  warning inside any hunk this change adds, `rustfmt --check` over every file it changes, and
  `npm test` in `src/vscode` (251 grammar tests)
- [x] 7.4 Load a real `.nx` file in the VS Code extension and verify hover renders highlighted NX
  inside `<div>` subtrees, at a member access, at a parameter, at a union case, and at a bare
  property value. Done at an editor, against the DrawnUI proposal sources. It confirmed the thing
  the substitute verification could not: hover popups honour the ```` ```nx ```` fence injection, so
  hover content is highlighted NX rather than monospace prose. It also found the one defect that
  only a person could see — a fragment carrying a parenthesized kind, `(property)
  ShapeCommon.shadows: Shadow[]?`, was barely highlighted, because the grammar's rich scoping is
  gated behind a declaration keyword and a line starting with `(` reaches none of it; worse, its
  annotation colon and nullable `?` were scoped as a ternary's and the word `type` inside
  `(primitive type)` as the `type` declaration keyword. Fixed by the `hover-annotation` rule in
  `src/vscode/syntaxes/nx.tmLanguage.json`, pinned by `src/vscode/test/grammar/hover-annotation.test.ts`
  and one case in `markdown-codeblock.test.ts` covering the injection path hover content travels,
  and required by the `editor-syntax-highlighting` delta this change now carries
- [x] 7.5 Update `src/vscode/CHANGELOG.md` and tick the "Improve hover … quality" item in
  `src/vscode/TODO.md:56` to reflect the hover half being done, and verify the entries describe the
  behavior a user sees
