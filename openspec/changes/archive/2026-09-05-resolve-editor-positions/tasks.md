## 1. Test scaffolding and a parity baseline

- [x] 1.1 Add a `position_for(source, marker)` test helper to the `tests` module in
  `crates/nx-language-service/src/lib.rs` that locates a cursor marker in a fixture and returns its
  `TextPosition`, so scenarios can be written against readable source instead of literal line and
  character numbers; verify by rewriting two existing completion tests to use it and confirming
  `cargo test -p nx-language-service` still passes
- [x] 1.2 Add a `assert_same_completions(single_line, multi_line, marker)` helper that resolves both
  spellings of a construct and asserts identical completion labels; verify it fails today on the
  `<Panel mode=⟨cursor⟩ />` fixture from `proposal.md` and passes on two single-line spellings
- [x] 1.3 Capture the working single-line behavior from the `proposal.md` measurement table as
  explicit tests — property-name completions, property-value member completions, supplied-property
  filtering, and hover on a declaration name; verify all four pass today, so a later regression on
  them is attributable

## 2. Failing tests for the specified scenarios

Write these before changing any behavior. Each must fail against the current implementation for the
reason recorded in `proposal.md`.

- [x] 2.1 Add tests for "Multi-line opening tag offers the same property completions as a single-line
  one" and "Multi-line opening tag offers contextual member completions" using the 1.2 helper; verify
  both fail today with the declaration-keyword list in place of the expected completions
- [x] 2.2 Add a test for "Supplied properties are recognized anywhere in the opening tag" with the
  supplied property on a different line from the cursor; verify it fails today
- [x] 2.3 Add a test for "Type completions are offered in a multi-line signature"; verify it fails
  today, and that it asserts the absence of declaration keywords as well as the presence of type names
  — **the premise did not hold**: `is_type_position` sees the annotation's `:` on the cursor's own
  line even when the signature spans lines, so this scenario already passes and the test is a
  regression guard. The layout dependence the requirement actually names was pinned instead by
  `a_colon_inside_a_property_value_does_not_make_a_type_position`, which does fail today
- [x] 2.4 Add tests for "Hover over a reference reports the referenced declaration" and "Hover over an
  expression reports its inferred type"; verify both fail today by returning no hover at all
- [x] 2.5 Add a test for "Hover over a component declaration reports its signature"; verify it fails
  today, which returns the symbol kind and name only
- [x] 2.6 Add tests for the conservative cases — "A position inside no identifiable construct yields
  no contextual result", "Hover on unknown syntax returns no result", and "Hover on an expression with
  no inferred type returns no result" using a document with a syntax error; verify they pass today and
  are therefore regression guards rather than new behavior
- [x] 2.7 Add malformed-input fixtures covering the states the design's first Risk enumerates — an unterminated tag,
  an empty property slot, `name=` with no value, and an annotation with no type — asserting only that
  each returns a result or no result without panicking; verify they pass today

## 3. Retain the analysis artifacts the resolver needs

- [x] 3.1 Change `WorkspaceDeclarations` (`crates/nx-language-service/src/lib.rs:900`) to retain the
  `ModuleArtifact` per module identity alongside the existing name and kind maps, per design D3;
  verify `cargo test -p nx-language-service` still passes and that
  `build_workspace_declarations` gained no additional call to `analyze_workspace_modules`
- [x] 3.2 Add an accessor from a `DocumentUri` to that module's `lowered_module` and `type_env`;
  verify with a test asserting a known component's parameter has a type in the retained environment
  — the reference is taken in a **function** body: a parameter interpolated into a *markup* body
  lowers with a span but reaches the type environment with no entry, a checking gap that closing
  would mean changing an analysis crate, which the Non-Goals forbid
- [x] 3.3 Confirm no analysis crate changed: verify `git diff --stat crates/nx-types crates/nx-hir
  crates/nx-syntax` is empty at this point, per the design's Non-Goals

## 4. The position resolver

- [x] 4.1 Add a private `positions` module in `crates/nx-language-service` defining the resolved
  context type — declaration, reference, component tag, property name, property value, type
  annotation, expression, or unresolved — per design D1 and D2; verify it compiles with an
  exhaustive match in a placeholder consumer
- [x] 4.2 Implement offset-to-node resolution over the queried document's syntax tree using
  `SyntaxTree::node_at` (`crates/nx-syntax/src/lib.rs:101`), parsing that one document per request
  per design D3; verify with tests that a cursor in a property value, a property name, a type
  annotation, and an expression each resolve to the corresponding context
  — **`node_at` could not be used.** It asks tree-sitter for the smallest descendant spanning the
  empty range at the offset, which refuses to enter a node the offset merely touches and refuses to
  enter a zero-width node at all. Both are where an editor asks: in `<Panel\n  mode=|\n/>` the
  cursor is at the end of `property_list` and inside a zero-width `rhs_expression`, and `node_at`
  answers `element`. `positions::ancestor_chain` descends with boundaries included instead, over
  the same public `SyntaxNode` API and with no change to `nx-syntax`
- [x] 4.3 Implement the element-interior rule from design D4 — inside an `element`, after the element
  name, before the terminator, and inside no child node resolves to a property-name context for that
  element; verify with the empty-slot-on-a-blank-line fixture that design D4 records, which a naive
  innermost-node implementation resolves to `element` and stops
- [x] 4.4 Read already-supplied properties from the element's `property_list` children rather than
  from line text; verify test 2.2 passes
- [x] 4.5 Attach semantics to a resolved context: resolve a component tag to its declaration through
  the document scope `document_scope` already builds, and resolve an expression position to an
  `ExprId` by innermost containing span over the lowered module's expression arena; verify with tests
  that a tag written under an import alias resolves to the aliased declaration, preserving the
  existing "Member completions are offered for an element reached through an import alias" scenario

## 5. Hover

- [x] 5.1 Rewrite `WorkspaceSnapshot::hover` (`crates/nx-language-service/src/lib.rs:316`) to consume
  the resolved context instead of matching against top-level document symbol selection ranges; verify
  tests 2.4 and 2.5 pass and test 1.3's declaration hover still passes
- [x] 5.2 Report the inferred type for an expression context by looking up the resolved `ExprId` in
  the module's `TypeEnvironment`; verify the parameter-in-a-body case from test 2.4 reports the
  parameter's declared type
- [x] 5.3 Report a declaration's signature for a declaration or reference context, reusing the
  `detail` that `declaration_from_item` (`crates/nx-language-service/src/lib.rs:1294`) already
  computes; verify test 2.5 passes and no new signature-formatting code was introduced
  — hover reuses `detail`, but `detail` for a component was `component <Panel />` and carries no
  properties, so the scenario's "SHALL include the component's properties and their declared types"
  could not be met by reuse alone. The one existing formatter was enriched (`markup_signature`)
  rather than a second one added, so completion detail and hover still agree by construction
- [x] 5.4 Return no hover where the context is unresolved or the type environment has no entry, per
  design D5 and D6; verify the 2.6 conservative tests still pass, including the literal case D6
  records as out of reach

## 6. Completions

- [x] 6.1 Derive the property-value member context from the resolved context instead of
  `property_value_context` (`crates/nx-language-service/src/lib.rs:1492`); verify the multi-line half
  of test 2.1 passes and the single-line baseline in 1.3 is unchanged
- [x] 6.2 Derive the property-name context from the resolved context instead of
  `component_property_context` (`:1555`); verify the other half of test 2.1 and test 2.2 pass
- [x] 6.3 Derive the type-annotation context from the resolved context instead of `is_type_position`
  (`:1647`); verify test 2.3 passes
- [x] 6.4 Delete `property_value_context`, `component_property_context`, `is_type_position`, and
  `supplied_properties` along with any helper left with no caller, per design D5 — no fallback path
  remains; verify `cargo build -p nx-language-service` reports no dead-code warnings and
  `grep -n "line_prefix" crates/nx-language-service/src/lib.rs` returns nothing

## 7. Integration and verification

- [x] 7.1 Update the `nx-lsp` delegation tests (`crates/nx-lsp/src/lib.rs:857`) for the richer hover
  content; verify `cargo test -p nx-lsp` passes and that `server_capabilities()` is unchanged, per
  the design's Migration Plan
- [x] 7.2 Re-run the `proposal.md` measurement table against a rebuilt `nx-lsp` over stdio and record
  the new results in the change's `review.md`; verify every row that read `null` or a
  declaration-keyword list now returns the specified result
- [x] 7.3 Run `cargo test --workspace` and `openspec validate resolve-editor-positions --strict`;
  verify both pass and that no test outside `nx-language-service` and `nx-lsp` needed changing

## 8. Hover at property names and type annotations

Raised as RF4 in `review.md`: the MODIFIED hover requirement lists a type annotation and a property
among the positions hover answers at, and the implementation returned nothing at both.

- [x] 8.1 Carry the name at the position through the resolved context — the property name written in
  the slot, absent at an empty slot, and the type name the cursor is on, absent where the annotation
  is still empty; verify the existing completion tests are unaffected, since neither field is read
  by a completion path
- [x] 8.2 Collapse `Declaration::properties` and `Declaration::property_types` into one
  `Vec<PropertyDeclaration>` carrying the property's name, the base type a namespace lookup takes,
  and the type as written; verify `property_value_members` still resolves a property's union through
  the declaring module and that the member-completion tests pass — the two parallel vectors could
  not answer hover, because `property_types` held `base_type_name`, which spells `string[]` as
  `string`
- [x] 8.3 Report the declared type at a property-name position and the named declaration at a type
  annotation, falling back to the primitive or built-in classification where the name is one NX
  defines rather than the workspace; verify with tests in both the single-line and multi-line
  spellings, and that an empty property slot and an empty annotation both still report nothing
- [x] 8.4 Show a property completion's declared type as its detail, in place of the constant
  "component property"; verify the property-completion tests pass — this falls out of 8.2 and is
  the point of it: hover and completion now read the one recorded type rather than two spellings
  that could drift
- [x] 8.5 Record the new positions as scenarios on the MODIFIED hover requirement in
  `specs/editor-language-service/spec.md`; verify `openspec validate resolve-editor-positions
  --strict` passes and `cargo test --workspace` is green

## 9. The position-to-expression query

Raised as RF7 in `review.md`. Amends the Non-Goal on analysis crates, per design D7.

- [x] 9.1 Add `LoweredModule::innermost_expr_at(within, offset)` and the `exprs()` iterator to
  `nx-hir`, so the query is answered where the arena and the span map live; verify with tests in
  `nx-hir` covering an enclosing expression losing to an inner one, an expression outside the
  bounding construct not answering, an expression with no recorded span not answering, and a
  boundary offset belonging to the expression it touches
- [x] 9.2 Replace the `0..expr_count()` walk in `inferred_type_hover` with that call, and rewrite
  the one test that rebuilt `ExprId`s from raw indices to use `exprs()`; verify
  `cargo test --workspace` passes and that `la-arena` is gone from
  `crates/nx-language-service/Cargo.toml`
- [x] 9.3 Record the amended Non-Goal and the reasoning as design D7; verify
  `openspec validate resolve-editor-positions --strict` passes

## 10. Literal spans

Raised as the first of the two limitations under "Findings not fixed" in `review.md`, and reachable
only after section 9 moved the position-to-expression query into `nx-hir`. Reverses design D6, per
design D8.

- [x] 10.1 Record the span of every literal at the point of lowering, routing the seven literal sites
  in `lower_expr` through one `literal_expr(literal, ty, span)` helper so a new literal kind cannot
  arrive without one; verify with an `nx-hir` test that a string, an integer, a float, a boolean, a
  null, and a negative literal each carry a span covering exactly what was written
- [x] 10.2 Fold `-` into a literal by rewriting the operand in place rather than allocating a second
  expression; verify with an `nx-hir` test that `-42` leaves exactly one literal in the arena
  spanning `-42` — a discarded operand is narrower than the folded literal and would shadow it in a
  lookup by offset, which is what left `-42` unanswerable
- [x] 10.3 Report the whole literal from the position resolver rather than the token under the
  cursor, since `-42` lowers with the span `-42` and a bound of `42` excludes it; verify hover
  answers at both the sign and the digits
- [x] 10.4 Record the new position as a scenario on the MODIFIED hover requirement and update the
  test that pinned the D6 limitation — a cursor in a quoted property value now reports `string`
  rather than nothing, and still must not report the type its enclosing element evaluates to;
  verify `cargo test --workspace` and `openspec validate resolve-editor-positions --strict` pass


## 11. Review fixes, second pass

Raised as RF10–RF16 in `review.md` against the section 10 work. No spec change: RF11 declines at a
position the literal scenario never covered, since the scenario is conditioned on a literal "written
in a position NX static analysis typed" and `null` infers as an unsolved variable.

- [x] 11.1 Report the whole literal wherever `-` was spelled as a prefix-unary expression, not only
  where it parsed as one `signed_numeric_literal`; verify hover answers for `{-42}`, `{-1.5}`, and
  the `-10` in `{x + -10}`, at the sign and in the digits both
- [x] 11.2 Decline a type that still carries an unsolved inference variable or an unresolved
  contextual name, not only `Unknown` and `Error`; verify `null` reports nothing rather than `T0?`,
  annotated and unannotated both
- [x] 11.3 Assert the content each type-annotation fixture must report rather than that it contains
  the word "type", and move the doc comment that had drifted onto it back to the empty-annotation
  test
- [x] 11.4 Record the positions hover still does not answer in `specs/future.md` against what the
  tree actually does: list elements, `null`, and a unit literal, alongside the three already there
- [x] 11.5 Trim the position resolver's literal kinds to the ones `grammar.js` produces, route the
  text-run literal through `literal_expr` so the span invariant holds at every literal site, and
  `cargo fmt` the files this change wrote; verify `cargo fmt --check` reports no diff in them
- [x] 11.6 Delete the list entry from `specs/future.md` — `[1, 2]` is not NX and hover answers
  inside the real spelling `{a b}` for literals and references alike — re-anchor the unit-literal
  entry on `{() 2}`, which parses clean and still reports nothing, and pin the list measurements
  with a test so the wrong conclusion cannot be re-derived
- [x] 11.7 Fail a hover or completion fixture that emits a `syntax-error` diagnostic, since hover
  declines inside a syntax error by design and the two are indistinguishable from a `None`; give the
  five fixtures that deliberately do not parse an explicit helper so the exception is stated at each
  site rather than implied
