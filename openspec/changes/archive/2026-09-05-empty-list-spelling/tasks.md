## 1. Grammar: admit the empty values brace

- [x] 1.1 In `crates/nx-syntax/grammar.js`, wrap the `choice` in `values_braced_expression` in
      `optional`, and regenerate. Verify the generated conflict set is unchanged from the current
      grammar's — compare `tree-sitter generate` output order-insensitively, since the warning list
      is not emitted in a stable order — so that admitting the empty form is confirmed to introduce
      no new ambiguity, per design D1.
- [x] 1.2 Add parser fixtures for `{}` in each accepting position: `let a:string[] = {}`,
      `let c = <Img fits={} />`, and `component <N /> = { <List>{}</List> }`. Verify each produces a
      `values_braced_expression` with no items and **no** `MISSING` node — today these parse only by
      error recovery inserting a zero-width `MISSING identifier`, so asserting the absence of that
      node is the real assertion.
- [x] 1.3 Add parser fixtures for the positions that still reject, per design D4:
      `component <N r:boolean /> = { <div>if r {} else { <B/> }</div> }`,
      `component <N /> = { <div>for x in xs {}</div> }`, and
      `component <N /> = { <p:html>Hi @{}</p> }`. Verify each still reports a parse error.
- [x] 1.4 Verify no existing form's parse changed: `{a}`, `{a b}`, `{(a + b) c}`,
      `{first second <Badge/>}`, `type T = { x:string }`, and an element-position
      `if r { <A/> } else { <B/> }` all produce the same trees as before the edit.

## 2. Type checking: type the empty braced expression

- [x] 2.1 Infer a zero-item `ValuesBracedExpression` as list-valued rather than scalar, taking its
      element type from the expected type at the site. Verify `let value:string[] = {}` infers
      `string[]` with no diagnostics, per design D2 and D3.
- [x] 2.2 Report a diagnostic when a zero-item braced expression has no expected type, naming the
      binding and directing the author to annotate. Verify `let value = {}` reports it and that the
      binding does **not** infer `object[]` — assert the inferred type explicitly, since falling back
      to `object[]` would otherwise pass a "no error" test, per design D3.
- [x] 2.3 Verify the empty form at a nullable list site produces a non-null empty list, not null:
      `type Brand = { links:ChatBrandLink[]? }` with `<Brand links={} />` yields a
      `ChatBrandLink[]` of length zero. This is the `T[]?` versus `T?[]` distinction
      `typed-braced-expression-kinds` requires be preserved.
- [x] 2.4 Confirm every binding site that supplies an expected type reaches 2.1 with it in hand —
      property bindings, property and field defaults, annotated `let`, function bodies with a
      declared return type, and element body content — and thread the expected type through any that
      do not. Verify with a test per site that `{}` is accepted there. This is the plumbing risk
      named in design's Risks.

      Two sites this task originally named were unreachable when it was written, because neither
      position admitted a `ValuesBracedExpression` at all. **Arguments** are now reachable and are
      covered by section 7. **List elements** are not, and stay that way by decision: a list item is
      a `value_list_item_expression`, which does not include the braced rule, so `{{"a"}}` and
      `{{}}` are syntax errors at every arity. See design D8 and proposal's "Not in this change".
- [x] 2.5 Verify the boundaries hold: `{1}` still infers `int` and not `int[]`, `{1 2 3}` still
      infers `int[]`, `{<A/> <B/>}` still infers `object[]`, and the scalar-to-list coercion at a
      list-typed site is unaffected.

## 3. Formatting: emit `{}`

- [x] 3.1 In `crates/nx-cli/src/format.rs`, emit `{}` for an empty list in property position, and
      remove `unspellable_empty_list` and its call site. Verify with a formatter test that a
      list-typed field holding no elements renders as `items={}`.
- [x] 3.2 Replace `test_format_empty_list_property_has_no_readable_spelling`, which asserts the
      failure this change removes, with a round-trip test: render a record whose list field is
      empty, parse and type check the output against the originating types, re-evaluate it, and
      assert a list with no elements comes back.
- [x] 3.3 Verify an empty list at a field whose declared default is non-empty still renders `{}` and
      re-reads as empty rather than as the default — the case that makes omission wrong as a general
      strategy.
- [x] 3.4 Verify the module doc comment at `format.rs:17`, which names the empty list as one of the
      values with no spelling, no longer says so, and that an action handler remains reported as
      unspellable.

## 4. Remove `void` from the source surface

- [x] 4.1 Remove `'void'` from `primitive_type` in `crates/nx-syntax/grammar.js` and regenerate.
      Verify `type Handler = { result:void }` no longer parses as a primitive type.
- [x] 4.2 Remove the `"void"` arm from `builtin_type` in `crates/nx-types/src/semantics.rs`, and
      update `test_builtin_types_resolve` accordingly. Verify `builtin_type("void")` returns `None`
      and that `type void = { value:int }` resolves a field declared `n:void` to the user record.
- [x] 4.3 Remove `"void"` from `PRIMITIVE_TYPE_COMPLETIONS` in `crates/nx-language-service/src/lib.rs`.
      Verify the completion list is the eight canonical names and offers no `void`.
- [x] 4.4 Verify a user-declared `type void` does **not** reach the primitive-name-to-host-type maps
      that still contain a `"void"` entry — `crates/nx-codegen/src/emit.rs` and
      `crates/nx-cli/src/typegen/languages/{csharp,typescript}.rs`. Generate C# and TypeScript for a
      module declaring `type void = { value:int }` and a field of that type, and assert the field is
      typed as the generated record and not as host `void`. This is the specific hazard named in
      design's Risks.
- [x] 4.5 Verify `Type::Void` is untouched: an `if` with no `else` still takes it, a block with no
      trailing expression still takes it, and `Type::void().to_string()` is still `"void"` so
      diagnostics keep naming it, per design D5.
- [x] 4.6 Verify nothing regressed by removing a name the corpus does not use: `cargo test` across
      the workspace, and the NX example corpus still compiles and evaluates.

## 5. Documentation and grammar references

- [x] 5.1 Update `nx-grammar-spec.md`: remove the `VOID ("void")` token, drop `"void"` from the
      `primitive_type` name list, and describe `ValuesBracedExpression` as admitting zero items while
      `ElementsBracedExpression` and `EmbedBracedExpression` do not.
- [x] 5.2 Update `nx-grammar.md`: remove `"void"` from the primitive alternatives and reflect the
      zero-item values brace.
- [x] 5.3 Verify the two documents agree with the regenerated grammar on both points, as
      `typed-braced-expression-kinds` requires of grammar references.

## 6. Verification

- [x] 6.1 Verify the `braced-value-sequences` delta's scenarios end to end: `{}` parses, type checks
      at an annotated site, evaluates to an empty list, formats back to `{}`, and re-reads as an
      empty list.
- [x] 6.2 Verify the `primitive-type-names` delta's scenarios: the primitive set is eight names,
      `void` in type position is an unresolved type, a user may declare `type void`, and the unit
      type still renders as `void` in a diagnostic.
- [x] 6.3 Verify the change is inert for existing source: the NX example corpus and the DrawnUI
      corpus compile, evaluate, and produce identical output before and after. One already-invalid
      sample is the sole exception; see 10.8.

## 7. Call arguments take a braced value

Added after sections 1-6, when task 2.4's finding that a call argument could not be written was
taken as something to fix rather than to scope out. See design D8.

- [x] 7.1 In `crates/nx-syntax/grammar.js`, draw `call_expression`'s arguments from a hidden
      `_call_argument` rule, `choice($.value_expression, $.values_braced_expression)`, and
      regenerate. Verify the conflict set is unchanged from before the edit, compared
      order-insensitively, so the narrow rule is confirmed to add no ambiguity.
- [x] 7.2 Add parser fixtures for a braced argument at each arity — `count({})`, `count({"a"})`,
      `count({"a" "b"})` — asserting the argument is a `values_braced_expression` with the expected
      item count and **no** `MISSING` node, and for a braced value in a later argument position
      beside an ordinary one: `pick({}, x, {"a" "b"})`.
- [x] 7.3 Add parser fixtures for what still rejects: `{{"a"} b}`, `{{} b}`, `items={{"a" "b"}}`,
      `items={{}}`, and a braced item inside a braced argument, `count({{"a"} "b"})`. Verify each
      still reports a parse error, so admitting the brace as an argument is confirmed not to have
      admitted it as an item.
- [x] 7.4 Verify the type checker needs no new plumbing: `infer_call` already checks each argument
      through `check_typed_binding_for` with the parameter type. Test that `{}` takes its element
      type from a parameter (`let value = {echo({})}` infers `string[]`), that `{"only"}` is
      accepted at a `string[]` parameter by the existing scalar coercion, that a braced argument is
      accepted in every position, and that a nullable list parameter accepts `{}`. Record-
      constructor call arguments turn out not to be type checked at all — `Row("x")` at an `int`
      field and `Row(1, 2)` at a one-field record are both accepted — so no assertion about the
      empty list there can fail. That gap is pre-existing and out of scope; it is pinned as a gap
      by `record_constructor_arguments_are_unchecked_including_the_empty_list` rather than
      asserted as empty-list behaviour.
- [x] 7.5 Discharge pending empty lists at `infer_call`'s three uncheckable-call exits — error
      callee, argument-count mismatch, non-function callee. Verify an undefined callee and a wrong
      argument count each report exactly one diagnostic, their own, and not also that the empty
      list's element type cannot be determined.
- [x] 7.6 Verify a non-list parameter reports only the mismatch, naming the parameter's type and
      not directing the author to annotate — the argument-site form of task 2.2's diagnostic.
- [x] 7.7 Verify end to end that a braced argument arrives as a list at each arity: `echo({})`
      evaluates to an empty list, `echo({"only"})` to a one-element list by parameter coercion, and
      `echo({"a" "b"})` to a two-element list.
- [x] 7.8 Update `nx-grammar-spec.md` and `nx-grammar.md` so `ParenFunctionCall`'s argument list
      reads `ValueOrValuesBracedExpression`, the nonterminal both documents already use for match
      and condition arms, and state that a braced value is still not a list item.

## 8. Close the positions the empty form reached beyond the design

Added after review, which found the form reaching sites sections 1-6 did not enumerate. The rule
they share is design D9; the two documentation listings are unrelated to it and ride along here.

- [x] 8.1 In `crates/nx-interpreter/src/interpreter.rs`, tell a body that produced no values apart
      from no body at all, so `<Box>{}</Box>` binds the empty list to the content property instead
      of leaving it unbound. Verify at a `T[]` content field, where the unbound property used to
      fail coercion, and at a `T[]?` one, where it used to read back as null — the `T[]?` versus
      `T?[]` rule the typing requirement states for property position. Verify `<Box />` still
      leaves the property to its declared default.
- [x] 8.2 Verify the interpreter and the code generator agree: the same source that evaluates to an
      empty list also generates TypeScript binding `[]`. Pinned by
      `an_empty_written_body_generates_an_empty_list_and_an_absent_one_takes_the_default` in
      `crates/nx-codegen/src/tests.rs`, which asserts both halves of the rule against both
      implementations from one source. The rule is keyed independently in each — the interpreter on
      whether a content expression was written, code generation on whether the emitted content list
      is non-empty, at three call sites across two crates — so a test rather than a manual check is
      what holds them together.
- [x] 8.3 Discharge and resolve pending empty lists on the multi-item content path in
      `check_content_binding`, and stop `normalized_sequence_type` joining an empty list's element
      variable into the sequence's item type. Verify `<List>{}<Badge/></List>` reports nothing at an
      `object[]` content site and at a `Badge[]` one, where the join would otherwise land on
      `object` and report a mismatch.
- [x] 8.4 Accept an empty list at a site that admits a list without being list-typed, resolving its
      element to `object`. Verify `type Box = { thing:object }` accepts both `<Box thing={} />` and
      `<Box thing={"a" "b"} />`, so the empty form is writable wherever the non-empty form is.
- [x] 8.5 Make an empty list absorb into the list it is joined with, and resolve the arm and branch
      bodies against the join. Verify `{if { c => {} else => {"a" "b"} }}` and
      `{if c {"a" "b"} else {}}` both check at a `string[]` return and evaluate to the empty list on
      the empty side.
- [x] 8.6 Discharge a `for` body's empty list at the `for`, so the enclosing site judges `{}[]` as a
      whole. Verify `let xs:string[][] = {for y in ys {}}` checks clean, `let xs:string[] = {for y
      in ys {}}` reports exactly one diagnostic, and `let a = {for y in ys {}}` still names the
      binding that needs the annotation.
- [x] 8.7 Render a type built around an empty list without naming the element variable at any depth,
      so the mismatch above reads `{}[]` rather than `T0[][]`. This is the depth-one rendering
      generalized; verify no `T<n>` reaches either diagnostic.
- [x] 8.8 Add parser fixtures pinning that `{}` parses in a value-position `if` branch, a
      value-position `for` body, and a condition arm body — all parse errors before this change —
      with no recovery node.
- [x] 8.9 In `crates/nx-cli/src/format.rs`, emit `{}` for an empty list on the own-value path and
      apply the nested-list guard there. Verify `format_value` of an empty list is `{}` rather than
      empty output, and that a list of lists is reported rather than flattened into a run of lines.
- [x] 8.10 Drop `void` from the two primitive alternations in
      `src/vscode/syntaxes/nx.tmLanguage.json` and from the primitive list in `README.md`, which
      section 5's document sweep missed. Verify no first-party listing of the primitive names still
      names it, by the word-level `*.md` sweep the `rename-primitive-types` change settled on —
      every `*.md` outside `node_modules`, `target`, and `openspec/changes/archive`. That sweep
      finds five more inventories, all now corrected: `src/vscode/README.md`, `nx-planning.md`,
      `nx-rust-plan.md`, `specs/001-nx-core-parsing/spec.md`, and
      `docs/drawn-ui-proposal-nx-enhancements.md`. The middle three are the same files that change
      had to be reopened over; scoping the sweep narrowly is what put them back out of step.
      Deliberately left, and not primitive inventories: `nx-rust-plan.md:509` (`const void* tree`,
      C), `nx-grammar-spec.md:267-269` (prose stating the new rule correctly), and the function-type
      examples in `docs/src/content/docs/` and `nx-planning-future.md` — see 10.9.
- [x] 8.11 Verify the change is still inert for existing source: run every `.nx` file in the
      repository before and after, and confirm identical output — the content-binding change in 8.1
      is the one edit here that can reach source containing no `{}`. The one file that does differ
      differs for a parser reason, not this one; see 10.8.

## 9. Settle the empty list without losing it (superseded by section 10)

Added after verification of section 8, which found two consequences of that pass: a demand that
could be dropped rather than met, and a meaning change wider than the artifacts described.

Tasks 9.1-9.4 are **superseded**: they built the settling rule that section 10 deletes outright by
making the empty list's type real. They are kept rather than removed because they record what was
tried and why it was not enough — the rule was correct and still needed a fifth case, then a sixth,
which is the argument section 10 rests on. Tasks 9.5 and 9.6 stand; they are about the interpreter
and the Migration Plan, and section 10 does not touch either.

- [x] 9.1 Replace the unconditional discharge at the arm join and the `for` body with a settling
      rule: where the enclosing type says what the elements are, the parts take it; where it cannot,
      the demand moves to the enclosing expression instead of being dropped. Verify
      `let f(c:boolean) = {if { c => {} else => {} }}` and `let f(ys:string[]) = {for y in ys {}}`
      each report exactly one diagnostic, where before this task they reported none.
- [x] 9.2 Verify the value cannot inhabit two list types unreported: a program binding such an `f`
      at both a `string[]` and an `int[]` site SHALL be rejected. This is the outcome D3 rejects the
      `object[]` fallback for, reached by a different route.
- [x] 9.3 Verify moving the demand costs nothing where the site does supply a type:
      `let both:string[] = {if c {} else {}}` and `let arms(x:boolean): string[] = {if { x => {}
      else => {} }}` are clean, and section 8's cases are unchanged — the arm join, the `for` body
      at a nested list site, the flat site's single diagnostic, and the named unannotated binding.
- [x] 9.4 Generalize the two discharge conditions in `check_typed_binding_for` from a depth-one
      empty list to one at any depth, so an expression the `for` now carries the demand for is
      settled by the site that accepts or rejects it.
- [x] 9.5 State the runtime rule on what it is actually keyed on: any body content that produced no
      values binds the empty list, not only `{}`. Add an interpreter test for a `for` over an empty
      list at a content property with a non-empty default — source containing no `{}` — and correct
      `normalize_content_values`'s doc comment, which named an untaken `if` as an example when an
      untaken `if` evaluates to null and never reaches that case.
- [x] 9.6 Correct the Migration Plan, which claimed every program that compiles today still means
      the same thing. Name the one shape that changes meaning and why keying on `{}` instead would
      be worse.

## 10. Give the empty list a real type

Sections 2 and 7-9 typed the empty list as a list whose element was an inference variable, and every
site that could conclude a binding had to resolve it by hand. Six of the twelve review findings were
that plumbing failing at a site nobody had listed. The variable was standing in for the bottom type;
this section makes it the bottom type. See design D10 and the rewritten D3.

- [x] 10.1 Add `Primitive::Never` in `crates/nx-types/src/ty.rs` with a `Type::never()` constructor,
      rendering as `never`. Verify the only exhaustive matches it breaks are the two host renderings
      in `crates/nx-codegen`, and fill both in.
- [x] 10.2 Infer a zero-item `ValuesBracedExpression` as `Array(Never)` rather than as a list with a
      fresh variable element, so it has its type without consulting a site.
- [x] 10.3 Add the two rules: `never` satisfies every expected type, and joining `never` with any
      type yields that type. Verify nothing else is needed to reach the sites — arrays are already
      covariant in `type_satisfies_expected`, and any array already satisfies `object`.
- [x] 10.4 Delete the machinery the variable required: `pending_empty_lists`, the end-of-analysis
      sweep and its caller in `check.rs`, `discharge_pending_empty_lists`, `resolve_empty_lists_in`,
      `settle_empty_lists`, the empty-list branches in `check_typed_binding_for`, the filter in
      `normalized_sequence_type`, and the resolution in `check_content_binding`. Verify the count
      drops from 45 references to the rendering helpers and the one diagnostic.
- [x] 10.5 Keep D3's diagnostic, now as a legibility rule rather than a typing failure, at the two
      places a binding's type can be fixed by an empty list: an unannotated value binding and an
      unannotated function return. Verify `let a = {}`, `let f(c:boolean) = {if { c => {} else => {}
      }}`, `let f(ys:string[]) = {for y in ys {}}` and `let <f /> = { }` each report exactly one
      diagnostic naming the binding.
- [x] 10.6 Verify `never` has no source spelling: it is absent from `primitive_type` in the grammar
      and from `builtin_type`, `type Handler = { result:never }` resolves the name as an ordinary
      undeclared reference, and `type never = { value:int }` may be declared and used.
- [x] 10.7 Verify every behaviour the earlier sections pinned is unchanged, by re-running the whole
      suite rather than by argument: all tests written against the variable-based machinery pass
      untouched, and the diagnostics for the `for`-body cases still spell the type `{}[]`.
- [x] 10.8 Verify the change is still inert for existing source: run all 111 `.nx` files in the
      repository from a `HEAD` worktree and from the working tree and diff the full output. **One
      file differs**, `src/vscode/samples/tally-survey.nx`, and it is invalid both before and after
      — it uses an unsupported positional attribute form, `<Option "Yes, borrowed"/>`. No valid
      program changes output, so the substantive inertness claim holds, but the claim is *not*
      "byte-identical" and must not be recorded as one. Its error recovery cascades differently:
      21 diagnostics before, 35 after, which is the whole of the corpus-wide 225 → 239. Admitting
      `{}` makes `{` followed by `}` a valid parse, so recovery takes a different path through the
      malformed input and the new lead diagnostic reports `Unclosed brace` at line 31 on a brace
      that is closed two lines later. That misleading message is a recovery-quality issue, not a
      defect in the new rule, so it is recorded rather than fixed: see "Brace Recovery Reports A
      Closed Brace As Unclosed" in `specs/future.md`, which carries the reproduction, the corpus
      measurement, and the note that the sample itself has never parsed.
- [x] 10.9 Record what the `void` sweep deliberately does not touch: the function-type examples that
      still write `void` as a return type — `docs/src/content/docs/reference/syntax/types.md:13`
      (`type EventHandler = (string) => void`),
      `docs/src/content/docs/overview/design-goals.md:39` and
      `docs/src/content/docs/tutorials/building-your-first-component.md:13`
      (`onClick:() => void`), plus the speculative uses in `nx-planning-future.md`. These are *not*
      broken by this change: NX has no function-type syntax at all, so
      `type EventHandler = (string) => string` and `onClick:() => object` are syntax errors today
      exactly as the `void` forms are. Verified by running each form through the working-tree build.
      Rewriting `void` there would substitute one unparseable spelling for another and imply a
      return type the language cannot express, so the examples are left as they are and the
      pre-existing documentation gap is named here rather than papered over.
- [x] 10.10 Make D3's two diagnostic sites recover the same way. The unannotated value binding
      returned `Type::Error` while the unannotated function return kept its inferred type, so
      `let a = {}` poisoned and `let f() = {}` did not, contradicting the comment at the value site
      saying the annotation is required "for legibility, not because the system cannot type it".
      Both now report and keep the type. Verify `let a = {}` reports once and `a` still satisfies
      both `string[]` and `int[]`, and that a genuine mismatch after it — `let s:string = {a}` — is
      still reported rather than suppressed by an error type. The function side recovers the same
      way but is not observable at a call site: an unannotated return arrives there as a bare type
      variable, which is a pre-existing defect unrelated to empty lists (`let f() = {"a"}` fails at
      a `string[]` site the same way on `0fc0463`). D3 states the rule and names that limit.
