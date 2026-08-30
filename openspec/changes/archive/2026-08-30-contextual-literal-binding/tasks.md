## 1. Grammar and syntax

- [x] 1.1 Add a `contextual_name` alternative to `rhs_expression` in `crates/nx-syntax/grammar.js`, defined as a single `identifier` and never a `qualified_name`; verify `fit=cover` parses to a CONTEXTUAL_NAME node and `fit=Fit.cover` and `fit=o.fit` are parse errors
- [x] 1.2 Order the new alternative after `literal` so `true`, `false`, and `null` keep lexing as `bool_literal` and `null_literal`; verify with a fixture asserting `flag=true` still produces a BOOL_LITERAL and not a contextual name
- [x] 1.3 Add a signed numeric literal alternative to `rhs_expression` covering `-` before `int_literal`, `real_literal`, and `hex_literal`, without modifying the literal token rules or `prefix_unary_expression`; verify `external component <C x: float64 = -1.0 />`, `type Opts = { x: float64 = -1.0 }`, and `<C x=-1.5 />` all parse, and that `flag=!true` is still rejected
- [x] 1.4 Add the same signed numeric literal alternative to `pattern`; verify `let classify(n: int) = {if n is { -1 => "neg one" 0 => "zero" else => "other" }}` parses, which is a parse error today
- [x] 1.5 Confirm expression positions and tokenization are unchanged; verify `a-1` and `a - 1` inside braces both still parse as binary subtraction, and `{-90 + currentRotation}` still evaluates to `-45`
- [x] 1.6 Regenerate the tree-sitter parser and run the existing corpus; verify `cargo test -p nx-syntax` passes with no fixture changing its parse tree
- [x] 1.7 Add `crates/nx-syntax/src/syntax_kind.rs` support for both new forms and refresh `node-types.json`; verify the kinds round-trip from the tree-sitter node names and that consumers can read the identifier text and span for a contextual name, and the sign and numeric token for a signed literal
  - **Amended.** This task originally also required typed wrappers in `crates/nx-syntax/src/ast.rs`. That file holds wrappers for exactly six top-level declaration constructs (`FunctionDef`, `Element`, `TypeDef`, `UnionDef`, `RecordDef`, `ComponentDef`); no expression- or literal-level node has one, not even `int_literal`, `string_literal`, or `identifier`. Generic `SyntaxNode` access is the established API at this level, and `crates/nx-hir/src/lower.rs` reads both new forms that way. Adding wrappers for only these two forms would have been inconsistent with every neighbouring node, so the `ast.rs` half was dropped rather than left silently unmet.
- [x] 1.8 Add valid fixtures for contextual names and signed literals in property value, property default, and pattern position, and invalid fixtures for the qualified, dotted, and `!`-prefixed forms; verify snapshots under `crates/nx-syntax/tests/snapshots` are regenerated and reviewed

## 2. HIR and lowering

- [x] 2.1 Add a distinct `Expr` variant for an unresolved contextual name in `crates/nx-hir/src/ast/expr.rs`, carrying name and span, and extend `Expr::span()`; verify `cargo build -p nx-hir` and that no existing match arm is left non-exhaustive
- [x] 2.2 Lower `rhs_expression` contextual names to the new variant in `crates/nx-hir/src/lower.rs`; verify a lowering test shows `fit=cover` producing the new variant rather than `Expr::Ident`
- [x] 2.3 Lower a bare pattern name to the new variant instead of `Expr::Ident` in match and property-list match arms; verify a lowering test covers both `if f is { cover => ... }` and the property-list fragment form
- [x] 2.4 Fold `-` applied directly to a numeric literal into a negative `Literal` during lowering in `crates/nx-hir/src/lower.rs` (~1013), in every position including expressions, with no new HIR node; verify a lowering test shows `= -1.0`, `= {-1.0}`, and `-90` inside `{-90 + currentRotation}` all producing the same negative literal
- [x] 2.5 Confirm folding applies only to a literal operand; verify `{-n}` still lowers to a negation applied to `n` and that `{-90 + currentRotation}` still evaluates to `-45`

## 3. Resolution and type checking

- [x] 3.1 Add `resolve_contextual_name(name, expected)` in `crates/nx-types/src/infer.rs` as the single resolution entry point: normalize the expected type by stripping nullability then one list level, then look the name up among enum members and payloadless union cases; verify unit tests for the enum, union, nullable, and list-typed cases
- [x] 3.2 Give the new node a pending marker in inference and resolve it at the element and component property binding site (~1602); verify the spec scenarios for enum-typed and union-typed properties type check and evaluate to the same value as the qualified form
- [x] 3.3 Resolve at the property and field default binding site (~1826); verify `external component <Img fit:Fit = cover />` and `type Opts = { fit:Fit = contain }` type check
- [x] 3.4 Resolve at the match pattern binding site (~581) with nominal-first precedence; verify exhaustiveness still holds for bare patterns and that `SaveState`-style wrong-union patterns stay rejected
- [x] 3.5 Verify negative literal patterns type check and match at runtime; assert `classify(-1)` returns `"neg one"` for the scenario in the spec
- [x] 3.6 Report a diagnostic when nominal resolution in pattern position displaces a visible lexical binding of the same name; verify a test with a `let idle` binding and a `LoadState` scrutinee emits the diagnostic and still resolves nominally
- [x] 3.7 Reject a pending node that reaches any site without an expected type with a clear diagnostic rather than a panic or silent `Type::Error`; verify with a test that constructs the case directly
- [x] 3.8 Suppress cascading errors: when the property or element is unknown, report only the unknown-property or unknown-element diagnostic and discard the pending node; verify a test asserts exactly one diagnostic

## 4. Diagnostics

- [x] 4.1 Add the unresolved-contextual-name diagnostic naming the expected type and listing its members, with an edit-distance suggestion; verify `fit=containt` reports `containt` is not a member of `Fit` and suggests `contain`
- [x] 4.2 Add the non-nominal-expected-type diagnostic directing the author to the quoted form; verify `alt=cover` at a `string`-typed property suggests `alt="cover"`
- [x] 4.3 Extend the enum-typed type-mismatch diagnostic to direct the author to the bare form; verify `fit="cover"` at a `Fit`-typed property is still rejected and now suggests `fit=cover`
- [x] 4.4 Add the payload-case diagnostic directing the author to element-style construction; verify `state=failed` for a payload case suggests `<LoadState.failed ... />`
- [x] 4.5 Add the parse-error guidance for the qualified unbraced form; verify `fit=Fit.cover` reports both `fit=cover` and `fit={Fit.cover}` as alternatives

## 5. Runtime

- [x] 5.1 Confirm a resolved contextual name evaluates through the existing enum and union case value paths with no new `Value` kind; verify an interpreter test in `crates/nx-interpreter/tests/` asserts `fit=cover` and `fit={Fit.cover}` produce equal values
- [x] 5.2 Verify canonical raw payload conversion and code generation output are byte-identical for the bare and qualified forms; assert this in a test rather than by inspection

## 6. Formatter

- [x] 6.1 Rewrite `format_attribute_value` in `crates/nx-cli/src/format.rs` (~214) to emit scalars unquoted: numbers as numeric literals, booleans as boolean literals, null as the null literal, and enum members and payloadless union cases as bare contextual names; verify a test asserts `<Box w=1.5 flag=true opt=null fit=cover />` for the case that currently prints every value quoted
- [x] 6.2 Emit float values with a real-literal spelling so they bind at float-typed sites; verify a `float64` field holding `-1.0` formats as `neg=-1.0` and not `neg="-1"` or `neg=-1`
- [x] 6.3 Add a round-trip test: format a value into NX source, re-parse and type check it against the originating types, and assert no diagnostics; verify it covers the float, boolean, null, enum, and union-case arms in one record

## 7. Language service and editor

- [x] 7.1 Add a property-value completion context in `crates/nx-language-service/src/lib.rs` (`completions`, ~334) offering members or payloadless cases of the property's declared type after `=`; verify a test asserts the member list and asserts lexically visible variables are excluded
- [x] 7.2 Return no contextual member completions when the expected type is not an enum or union, or when the element or property is unknown; verify with a test
- [~] 7.3 **Not applicable.** The language service has no go-to-definition, and `hover` resolves only declaration symbols, never references (`crates/nx-language-service/src/lib.rs`, `hover`). Making a bare name hoverable means building reference resolution, a new language-service feature outside this change; no spec requirement depends on it.
- [~] 7.4 **Not applicable.** The language service has no rename operation at all, so there is nothing for bare occurrences to be included in. Same reason as 7.3.
- [x] 7.5 Add TextMate scoping for the bare value token in `src/vscode/`; verify by inspecting a sample file that the bare value is scoped distinctly from a quoted string

## 8. Documentation

- [x] 8.1 Update `nx-grammar.md` (`RhsExpression`, `Pattern`) and `nx-grammar-spec.md` with the new production, the AST node, and a resolution paragraph beside the existing MemberAccess disambiguation (~768); verify the productions match the shipped grammar
- [x] 8.2 Amend NXE2 change 2 in `docs/drawn-ui-proposal-nx-enhancements.md` from the quoted form to the bare form with the strict-split rationale, and update NXE8 to record that its `RhsExpression` member-access relaxation is superseded, its prefix-negated-literal half is delivered, and its integer-widening half remains open; verify all three findings read consistently with what shipped
- [x] 8.3 Update the §8.1 worked example and Appendix A defaults in `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` to the bare form; verify the example still type checks
- [x] 8.4 Add contextual literals to the NX examples under `examples/nx/**`; verify the examples build and run

## 9. Verification

- [x] 9.1 Run the full workspace test suite and verify `cargo test` passes with no pre-existing test modified except the formatter attribute test
- [x] 9.2 Walk every scenario in `specs/unbraced-literal-forms/spec.md` and confirm each has a corresponding test; verify the three delta specs' new scenarios are covered too
  - **Reopened, then closed.** The `enum-values` requirement *Enum members are referenceable without naming the enum type* had two scenarios with no test, and they did not pass. The requirement is now stated as what holds — property types resolve in the declaring module's namespace, and a member whose enum is not nameable here reports the needed import — and its three scenarios are covered by cross-module tests in `crates/nx-api/src/artifacts.rs`, which `crates/nx-types/tests/contextual_literals.rs` could not express because `check_str` is single-module. Carrying a resolved member through lowering without an import is deferred to the nominal-identity change (RF1 in `review.md`).
- [x] 9.3 Confirm no serialized output changed: run code generation and canonical payload conversion over the example corpus before and after and verify the outputs are identical
- [x] 9.4 Scope the delta specs to what this change delivered, so archiving promotes only statements that hold: `discriminated-unions` drops the unconditional lexical-scope clause and states the same lowering deferral `enum-values` states; `editor-language-service` records that completion lookup is by declaration name over the workspace snapshot rather than through the import graph; `unbraced-literal-forms` scopes the round-trip guarantee to values whose fields are all scalars. Each is restored or generalized by a successor change — see `review.md`, Review-fix pass
  - **Two further narrowings after verification.** `enum-values` records that two same-named enums are told apart by their declared members rather than by origin, so same-named enums declaring the same members are not distinguished (RF1); `discriminated-unions` records that union resolution reaches its definition by name and prefers a local union, so same-name isolation is enum-only (RF6). `nominal-type-identity` closes both.

## 10. Cross-module resolution

- [x] 10.1 Resolve a declaration's property type references in the module that declares them, for imported and peer components, functions, and records; verify an imported component's enum-typed property resolves to the declaring module's enum
- [x] 10.2 Reject a same-named local *enum* whose members differ from the declaring module's, standing in for it at an imported property; verify both the bare and qualified forms are reported
  - **Narrower than first claimed.** Two enums are compared by `{name, members}`, so a local enum declaring the same members is still accepted (value-identical today, since a member carries only its name). Unions are not covered at all: resolution reduces the resolved type to its name and re-looks-it-up local-first, so any same-named local union stands in. RF1 and RF6 in `review.md`; both close in `nominal-type-identity`
- [x] 10.3 Report a resolved member whose nominal type is not nameable in the using module, instead of lowering it to an unresolvable reference; verify no `unresolved:` slot reaches generated IR
- [x] 10.4 Cover the cross-module cases with workspace tests; verify the selective, component-only, same-name-collision, and wildcard-alias forms each have a test
