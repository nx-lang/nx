## Context

See `proposal.md — Why` for motivation and the measured behavior tables.

Three facts about the current implementation shape the approach:

- `WorkspaceDeclarations` already keeps a `ModuleArtifact` per module identity
  (`crates/nx-language-service/src/lib.rs:1120`), and every `Declaration` already carries an
  `origin: (module identity, LocalDefinitionId)` (`lib.rs:1019`). A hover can therefore reach the
  HIR item behind any visible name without new plumbing.
- `Declaration::detail` is completion detail. It is a single line rendered inline in a completion
  list and as a `DocumentSymbol` detail, and `declaration_hover` (`lib.rs:1064`) reuses it as a
  hover body. That reuse is the whole reason hover shows non-NX spellings.
- `positions::name_context` (`positions.rs:379`) is the catch-all: any `IDENTIFIER` that no earlier
  rule claimed becomes a `Reference`. Parameters, record fields, union cases, and member-access
  members all fall through to it and are then looked up as if they were top-level names, which they
  are not.

## Goals / Non-Goals

**Goals:**

- Type inference reaches every expression in a module, so that both the checker and hover stop going
  dark inside intrinsic markup.
- Hover distinguishes a name's *declaration site* from a *use* of it, and answers at both.
- Hover content is fenced NX, produced by a renderer that is hover's own rather than borrowed from
  completions.
- The conservative contract is preserved exactly: a position that genuinely has no metadata still
  returns nothing, and the asymmetric-test discipline in `lib.rs:2028` still holds.

**Non-Goals:**

- Changing what completions or document symbols show. The NX spellings are built as shared helpers
  so those could adopt them later, but this change does not alter `Declaration::detail`.
- Inventing a binding contract for an unresolved tag. An unknown tag stays unknown; only its
  children get checked.
- Any new LSP method or capability.

## Decisions

### D1 — The inference fix is a recursion, not a synthesized contract

`infer_element_expression` (`crates/nx-types/src/infer.rs:1332`) ends at `infer.rs:1459` with
`self.nominal_named_type(&element.tag)` for a tag that resolves to nothing. That fallthrough infers
each expression in `element.content` and each property value, discards the resulting types, and
returns the same nominal type it returns today.

*Alternative considered:* build a permissive `ElementBindingSpec` for unknown tags so the existing
`check_element_bindings` path runs. Rejected — that path exists to check bindings against a declared
contract, and an unknown tag has none. Running it would either invent property names or emit
"unknown property" noise for every intrinsic attribute in the workspace, which is a separate
question this change deliberately does not open.

*Consequence, deliberately accepted:* the four resolved-tag paths already infer their children as a
side effect of checking bindings, so this makes the unresolved path behave like the others rather
than adding a new behavior. The recorded types then flow to hover for free, because
`inferred_type_hover` (`lib.rs:432`) already reads `type_env`.

### D2 — Non-top-level declarations are read from HIR through `origin`, not cached on `Declaration`

A parameter's type, a record field's type, a union case's payload, and a payload field's type are
four more lists that could be added to `Declaration`. They are not. The resolver reports the
*owning* declaration's name alongside the position, hover looks the owner up in `scope.visible`, and
then reads the owning `Item` out of `artifacts[origin.0]` by `origin.1`.

*Why:* `Declaration` is the completions and symbols projection. It carries `properties` and
`members` because completions offer them one name at a time, and `members` is already filtered to
payloadless cases (`lib.rs:1657`) for that reason — which makes it the wrong source for a union
hover that must list every case. Growing it with three more hover-shaped lists would create a second
projection inside the first. Reading HIR through `origin` gives every hover the declaration exactly
as written, once.

### D3 — One resolver variant for declaration sites, one for members

`PositionContext` gains two cases rather than five:

- `MemberAccess { member, span }`, where `span` is the range of the **whole** member-access
  expression, not of the member identifier. This is what makes the type lookup succeed:
  `innermost_expr_at` (`crates/nx-hir/src/lib.rs:944`) requires `within.contains_range(span)`, and
  the member identifier's own range cannot contain the access it is part of. That span mismatch is
  the entire cause of the missing member hover.
- `LocalDeclaration { kind, name, owner, span }`, where `kind` distinguishes a parameter, a record
  field, a union case, and a union payload field, and `owner` names the declaration it is written
  in. Hover switches on `kind` to decide which part of the owning item to read.

*Alternative considered:* a distinct variant per site. Rejected — `hover_contents` already has one
arm per variant, and four near-identical arms differing only in which HIR list they index is the
shape a `kind` field exists to avoid.

### D4 — A written property value is a name, and its span is the name's

`PositionContext::PropertyValue` currently reports a zero-width span at a slot that has a bare name
written in it, and `hover_contents` returns `None` for it unconditionally (`lib.rs:416`). Both are
corrected: the variant carries `name: Option<String>` and the written name's real span, hover
resolves it through the same `property_value_members` path completions use, and only `name: None`
reports nothing.

This keeps one rule for bare names: `<Card role=admin />` and `let r: Role = admin` are the same
question asked in two places, and after this change they give the same answer.

### D5 — Annotations are read as written; only unannotated declarations consult inference

Where a declaration carries a written type annotation, hover shows that annotation's spelling. Where
it does not, hover reads `type_env`. A written `UserId` is not silently expanded to `int`, because
the author chose the alias and the alias is what a reader wants to see; an unannotated
`let value = 42` has no spelling of its own, so the inferred one is the only thing to show.

### D6 — Hover gets its own renderer; `detail` is left alone

A new rendering layer produces the fenced block. It is fed by small helpers that spell NX
declarations — `let add(count:int): int`, `type Size = int`, `let <Panel title:string />`,
`let num: UserId` — and those helpers are where a later change would point completion detail and
document-symbol detail if that is wanted. This change does not point them there.

*Why not now:* completion detail is rendered inline in a list next to the label, and document-symbol
detail is rendered in a breadcrumb. Both are single-line, space-constrained surfaces where the
current terse spellings are arguably correct; hover is neither. Changing all three at once would
also churn the completion and symbol tests for reasons unrelated to hover, which is what this change
is about.

### D7 — The kind is a parenthesized prefix, borrowed from TypeScript

NX has no spelling for "a parameter, in isolation". Rather than invent one, hover writes
`(parameter) count: int` and `(property) User.name: string` inside the fence — TypeScript's
convention, which every editor user already reads fluently, and which is visibly *not* NX syntax so
it cannot be mistaken for one. A declaration that *does* have an NX spelling shows that spelling
alone, with no prefix and no repeated kind line.

## Risks / Trade-offs

- **Newly surfaced type errors could be extensive.** `examples/nx/`, `sample-apps/`, and the DrawnUI
  catalog nest almost everything inside intrinsic elements, and none of that has ever been type
  checked. → Measure before building anything else: the first task lands the inference recursion
  behind nothing and records the full diagnostic delta across the repo. If the delta is large or
  contains checker defects rather than source defects, that is discovered before the hover work is
  built on top of it, and the change can be split at that point rather than after.
- **A newly surfaced diagnostic might be spurious** — an expression lowering records inside an
  unresolved element that has no meaningful type on its own. → The spec pins
  "an unresolved tag reports nothing beyond the tag itself"; any spurious diagnostic fails that
  scenario and is a defect to fix, not a result to accept.
- **Fenced NX degrades in clients with no NX grammar.** A client that does not know the `nx` tag
  renders the block as unhighlighted monospace. → That is still strictly better than today's
  proportional prose, and the one client that ships today (`src/vscode`) already has the grammar.
- **More expressions inferred per module.** Bounded by the amount of markup in the module, and the
  work is the same inference the resolved-tag paths already do. No new analysis pass is introduced
  and the snapshot's `OnceLock` still runs analysis once.
- **Reading HIR through `origin` at hover time** costs a map lookup and an item index per request
  rather than a field read. Hover is a per-keystroke-idle request on one position; the artifact is
  already in memory.

## Migration Plan

No data or protocol migration. The source migration is the newly surfaced diagnostics: they are
fixed in `examples/`, `sample-apps/`, and the catalog as part of this change, and each fix is
recorded as a source defect the checker was hiding or as a checker defect the recursion exposed.

## Open Questions

- Whether the newly surfaced diagnostics include cases where a *correct* program is rejected because
  inference is weaker inside markup than the resolved-tag paths assume. Deferrable: the measuring
  task answers it, and the answer changes which fixes are needed, not the specs, the approach, or
  the task breakdown.
