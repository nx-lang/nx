## Context

See `proposal.md` — Why for the motivation and the measured failures.

Three facts about the existing code shape this design:

- **The syntax tree is discarded after lowering.** `analyze_module` (`crates/nx-types/src/check.rs:312`)
  moves `parse_result.tree` into `lower(...)` and drops it; `ModuleArtifact` records only
  `parse_succeeded`. Nothing downstream of analysis can look at syntax.
- **The lowered module and type environment survive but are thrown away by the language service.**
  `ModuleArtifact` carries `lowered_module` and `type_env`; `build_workspace_declarations`
  (`crates/nx-language-service/src/lib.rs:508`) reads names and kinds out of them and keeps neither.
- **`nx-syntax` already has the position primitive.** `SyntaxTree::node_at(offset)`
  (`crates/nx-syntax/src/lib.rs:101`) returns the innermost node at a byte offset, and `SyntaxTree`
  owns its source through an `Arc<String>`, so it is self-contained rather than borrow-tied.

Tree-sitter's error recovery is strong enough for the incomplete states editors ask about. Parsing
`let two = <Panel\n  mode=\n/>` — a multi-line tag whose property value has not been typed yet —
produces a complete path to the cursor in 0.09 ms:

```
(element
  name: (element_name (identifier [2,11]-[2,16]))          ; Panel
  properties: (property_list
    (property_value
      name: (qualified_markup_name [3,2]-[3,6])            ; mode
      value: (rhs_expression [3,7]-[3,7]
        (contextual_name [3,7]-[3,7])))))                  ; MISSING identifier, at the cursor
```

The element name and the property name are both reachable from the cursor's node, across lines.

## Goals / Non-Goals

**Goals:**

- One resolution step that both hover and completions consume, so the two cannot disagree about what
  the cursor is on.
- Context that is a function of the analyzed document, not of line layout.
- No change to the shape of `ModuleArtifact` or to any analysis crate's behavior.
  - **Amended during review.** `nx-hir` gained one read-only query, `LoweredModule::innermost_expr_at`,
    and the `exprs()` iterator it is built on (D7), and lowering now records the span of every
    literal (D8). No analysis result and no existing signature changed: a literal's span was absent
    before and is present now, and nothing but a lookup by offset reads it.

**Non-Goals:**

- Caching or incrementality across requests. Snapshot construction cost per request is what it is
  today; see `proposal.md` — Impact.
- A resolution result rich enough for go-to-definition or rename. Those want a definition identity
  for every reference; this change needs only enough to describe hover and completion context.
  The resolver should not be shaped to preclude that, but nothing is added for it here.

## Decisions

### D7: The position-to-expression query belongs to `LoweredModule`, not to its callers

Raised as RF7 in `review.md` and decided after the first implementation. The language service located
an expression by walking `0..expr_count()` and rebuilding each `ExprId` with `ExprId::from_raw`. That
is a reach around `LoweredModule` rather than a use of it: the arena and the span map are private, so
`expr_count` plus raw-index reconstruction was the only way to ask "which expression is at this
offset?" from outside. The query moved to `nx-hir` as `innermost_expr_at(within, offset)`, with an
`exprs()` iterator replacing the raw-index reconstruction. `nx-language-service` no longer depends on
`la-arena`.

The search stays linear. Modules hold hundreds of expressions and this runs once per position query,
so an index is not yet worth its invalidation surface — and an interval query that is genuinely
sublinear in the worst case needs an interval tree, because lowered spans neither nest reliably nor
are all present. What the move buys is that this is now a decision `nx-hir` can revisit alone: the
index, when it is warranted, goes behind this signature and no caller changes.

The same holds for the two limitations recorded under "Findings not fixed" in `review.md` — a literal
carries no usable span, and a parameter interpolated into a markup body reaches the type environment
with no entry. Both are now fixable entirely within the analysis crates, because the language service
no longer knows how an expression is found.

### D1: Resolve syntax first, then consult semantics — do not resolve from the HIR alone

The resolver maps an offset to a *context* using the CST, then attaches meaning to that context from
the lowered module and `TypeEnvironment`.

Resolving from the HIR alone cannot work: a property name inside an opening tag, a type annotation,
and a component tag name are not expressions and have no `ExprId`. Only the syntax tree distinguishes
"cursor is at a property-name position" from "cursor is in an expression".

Resolving from the CST alone cannot work either: it gives no types, and it cannot tell which
declaration a tag resolves to through the import graph — which the completion requirements already
depend on (`editor-language-service`, "Element and property lookup SHALL follow the import graph").

So: the CST names the context, the HIR and type environment fill it in.

*Alternative considered:* extend the HIR to carry every syntactic position. Rejected — it would put
editor concerns into the lowering pipeline and change a structure the whole compiler depends on, to
serve one consumer.

### D2: The resolver lives in `nx-language-service`

A new private module in `nx-language-service`, not in `nx-hir` or `nx-syntax`. Position-to-context
resolution is an editor concern with no compiler consumer, and `nx-language-service` is already where
editor-facing projection lives.

*Alternative considered:* `nx-hir`, so a future non-LSP host could reuse it. Rejected as premature —
a future host reaches the resolver through `nx-language-service` anyway, which is the API that
change would expose.

### D3: Re-parse the queried document rather than retain trees in `ModuleArtifact`

The language service parses the one document a request names, and reads the lowered module and type
environment for it out of the retained `ModuleArtifact`s.

Retaining `SyntaxTree` in `ModuleArtifact` is mechanically easy — it is self-contained. It is
rejected because it grows a structure every consumer of the compile pipeline allocates (CLI, codegen,
FFI, the .NET and Node bindings) to serve editor requests alone, and because it would make an
editor-only need a permanent property of the analysis contract.

The cost of the alternative is one redundant parse per position request: 0.09 ms measured on the
multi-line fixture above, against a snapshot build that already type checks the whole workspace.

*Consequence:* `WorkspaceDeclarations` must retain the `ModuleArtifact`s it currently consumes and
drops. It is already computed at most once per snapshot under a `OnceLock`, so this changes what the
snapshot holds, not how often it is built.

### D4: An element-interior position that matches no child node is a property-name position

`node_at` returns the innermost node *covering* the offset, and an empty slot in a multi-line tag is
covered by no child. Parsing `let two = <Panel\n  mode="a"\n  \n/>` puts `property_list` at
`[2,2]-[2,10]`; a cursor on the blank line at `[3,2]` is inside `element` (`[1,10]-[4,2]`) and inside
nothing else.

The resolver therefore treats "inside an `element`, after the element name, before the terminator,
and inside no child node" as a property-name context for that element, and reads already-supplied
properties from the element's `property_list` children — the whole list, not the text following the
tag name on one line, which is what `supplied_properties`
(`crates/nx-language-service/src/lib.rs:1592`) does today.

A naive innermost-node implementation would return `element` and stop, and the empty-slot case — the
most common place to ask for a property completion — would regress.

### D5: An unresolved position yields no result, and that is what makes the replacement safe

The line-scanning helpers are removed rather than kept as a fallback. Keeping them would mean two
sources of context that disagree, and the disagreements would be invisible.

This is safe because the conservative contract is already specified: hover returns nothing when it
cannot determine useful information, and the property-value requirement already says an unknown
element or property offers no member completions. A position the resolver cannot classify falls back
to the general completion set, exactly as an unrecognized position does today.

### D6: Hover on a literal is out of reach in this change

`Expr::span()` returns a zero-width span for literals — `Expr::Literal(_) => TextSpan::new(0, 0)`,
commented "Literals don't track spans yet" (`crates/nx-hir/src/ast/expr.rs:330`). A literal therefore
cannot be found by offset in the expression arena, so its inferred type cannot be reported.

Hover returns no result there, which the conservative contract already permits. Giving literals real
spans is a change to lowering and belongs to its own proposal; the specs added here deliberately ask
for inferred types on references and parameters, not on literals.

**Reversed during review — see D8.** Once D7 moved the position-to-expression query into `nx-hir`,
recording the spans was a change to one crate rather than a reach across two, and it turned out to be
seven call sites.

### D8: Literals record the span they were written at

Reversing D6, and only reachable because D7 had already moved the query. `Expr::Literal` still
carries no span in the enum — adding one would change a shape the whole compiler matches on — so a
literal is located the way an identifier already is: through `LoweredModule`'s span map, written at
the point of lowering. `Expr::span()` is untouched, and the seven literal sites in `lower_expr` route
through one `literal_expr(literal, ty, span)` helper so a new literal kind cannot quietly arrive
without a span.

Two consequences worth recording:

- **Folding `-` into a literal now rewrites the operand in place** rather than allocating a second
  expression. With spans, a discarded operand stops being invisible: nothing references it, but a
  lookup by offset still finds it, and being the narrower of the two (`42` inside `-42`) it would
  shadow the folded literal — which is the one the checker typed. In-place folding is what makes
  `-42` answer at all.
- **The position resolver reports the whole literal, not the token under the cursor.** `-42` lowers
  with the span `-42`, so a bound of `42` would exclude it. A literal is one expression however many
  tokens spell it.

What this does *not* reach is a literal the checker never visited — markup body content, which is
inferred only where the tag declares a content property. Hover on the text in `<div>hello</div>` still
reports nothing, for the same reason a parameter interpolated into a markup body does. That is the
remaining half of "Findings not fixed" in `review.md`, and it is a change to what the checker visits
rather than to what lowering records, so it stays out of this change.

## Risks / Trade-offs

- **The design leans on tree-sitter error recovery, which is not guaranteed for every malformed
  state.** → The scenarios that matter are enumerated and tested directly as fixtures: an unterminated
  tag, an empty property slot, a property with `=` and no value, a type annotation with no type. Where
  recovery does not produce a usable path, D5 makes the outcome "no contextual result", which is the
  specified behavior rather than a wrong answer.
- **Replacing the heuristics can regress cases they currently get right.** → The single-line cases in
  the measurement table are working behavior. Each becomes a parity test asserting the single-line and
  multi-line spellings of the same construct produce identical completions, so a regression on either
  fails.
- **The type environment may have no entry for a resolvable position.** → Hover returns no result
  rather than an invented one (D5, D6). The added scenario "Hover on an expression with no inferred
  type returns no result" pins this.
- **`WorkspaceDeclarations` grows to hold the artifacts.** → Bounded by the documents in the snapshot,
  which the snapshot already holds source for, and freed with it. No change to how often analysis runs.
- **Two parses of the queried document per position request.** → Accepted, and quantified in D3.
  If it ever matters, D3 reverses into retaining the tree without changing the resolver.

## Migration Plan

No data or API migration. `nx-lsp` advertises the same capabilities and its handlers keep their
signatures, so the VS Code extension picks the behavior up when the server binary is rebuilt; the
existing delegation tests in `crates/nx-lsp/src/lib.rs` are updated for richer hover content rather
than for a new surface.

Sequencing: implementation waits for `empty-list-spelling` to complete. That change alters the
primitive type set and the declaration keyword completions, both of which the completion scenarios
here read; starting after it avoids writing fixtures against a keyword list that is about to move.

Rollback is reverting the change — no persisted state and no protocol version is involved.

## Open Questions

- Whether hover should render types through the same formatter diagnostics use, or its own. Deferrable:
  it changes the text inside a hover, not whether a hover is produced or what the scenarios assert.
