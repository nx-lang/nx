## Why

The `resolve-editor-positions` change gave hover a semantic position resolver, and hover now answers
at references, expressions, literals and tags. Measuring the shipped behavior across the constructs
NX is actually written in shows two remaining problems: hover is silent in the places NX code
mostly lives, and where it does answer, it answers in prose rather than in NX.

**Hover is silent inside markup.** An element whose tag resolves to no declaration — `<div>`, or any
intrinsic or not-yet-written tag — never has its content or property values inferred.
`infer_element_expression` (`crates/nx-types/src/infer.rs:1332`) dispatches on function, component,
record and union-case tags; the unresolved fallthrough at `infer.rs:1459` returns
`self.nominal_named_type(&element.tag)` without visiting `element.content` or the property values.
Lowering *does* record spans for those child expressions, so `innermost_expr_at`
(`crates/nx-hir/src/lib.rs:944`) finds the `ExprId` — `TypeEnvironment::get_expr_type` simply has no
entry for it, and hover's conservative rule then correctly declines.

The same hole is a silent type-checking gap, which is the more serious half of it. Name resolution
reaches into these children, but inference does not, so wrapping anything in a `<div>` disables type
checking for the whole subtree:

| Source | Diagnostics today |
| --- | --- |
| `let r() = { 1 + "a" }` | `Binary operator Add cannot be applied to types int and string` |
| `let r() = <div>{1 + "a"}</div>` | *none* |
| `let r() = <Wrap content={42} />` | `Property 'content' on 'Wrap' expects string, found int` |
| `let r() = <div><Wrap content={42} /></div>` | *none* |
| `let r() = <Wrap>{1 + "a"}</Wrap>` (declared content property) | `Binary operator Add …` |
| `let r() = <div>{nope}</div>` | `Undefined identifier 'nope'` — resolution reaches, inference does not |

**Hover is silent at three more classes of position**, measured against the current implementation:

| Position | Result today | TypeScript's answer for the analogue |
| --- | --- | --- |
| `u.na⟨cursor⟩me` — a member-access field | *none* | `(property) User.name: string` |
| `let g(cou⟨cursor⟩nt:int)` — a parameter declaration | *none* | `(parameter) count: number` |
| `let <Row cou⟨cursor⟩nt:int />` — a markup parameter | *none* | — |
| `type U = { i⟨cursor⟩d: int }` — a record field declaration | *none* | `(property) U.id: number` |
| `type Role = ad⟨cursor⟩min \| guest` — a union case declaration | *none* | `(enum member) Role.admin` |
| `\| failed { mess⟨cursor⟩age:string }` — a payload field | *none* | — |
| `<Card role=ad⟨cursor⟩min />` — a bare property value | *none* | — |
| `{Role.ad⟨cursor⟩min}` — a qualified case | *none* | — |

The declaration sites are worse served than their uses: `role=admin` at a typed `let` site reports
`` `Role.admin` ``, while the case's own declaration reports nothing. The member-access failure is a
span mismatch rather than missing analysis — `name_context` (`crates/nx-language-service/src/positions.rs:379`)
classifies the field as a `Reference` spanning only the field identifier, and `innermost_expr_at`
requires `within.contains_range(span)`, which the enclosing `u.name` expression cannot satisfy.

**What hover does return is not rich and is not NX.** Every result is an inline-code span followed by
a plain paragraph, so the signature renders in proportional body text with no highlighting:

| Position | Contents today |
| --- | --- |
| `let ad⟨cursor⟩d(count:int): int` | ``function `add` `` + `function add(count: int): int` |
| `let va⟨cursor⟩lue = 42` | ``value `value` `` — no type, though inference has one |
| `type Mo⟨cursor⟩de = light \| dark` | ``union `Mode` `` — the cases are not shown |
| `type Us⟨cursor⟩er = { id:int name:string }` | ``record `User` `` — the fields are not shown |
| `type S⟨cursor⟩z = int` | ``type `Sz` `` + `type = int` |
| `let n⟨cursor⟩um: UserId = 1` | ``value `num` `` + `value: UserId` |

Three problems compound there. The second line is not valid NX — there is no `function` keyword
(`let add(count:int): int`), `type = int` is not a spelling at all, and `value: UserId` is not one
either; these strings are `Declaration::detail`, built for completion detail by
`markup_signature` (`crates/nx-language-service/src/lib.rs:1704`) and `function_signature`
(`lib.rs:1718`) and reused verbatim by `declaration_hover` (`lib.rs:1064`). The kind is stated twice.
And the VS Code extension already ships a markdown fence injection grammar for NX
(`src/vscode/syntaxes/nx.markdown.codeblock.tmLanguage.json`, matching ```` ```nx ````) that hover
never uses, so highlighting is available today and left on the table.

## What Changes

- **Type inference visits element content and property values for every element, including tags that
  resolve to nothing.** `infer_element_expression`'s unresolved fallthrough infers each content
  expression and each property value before returning the nominal type for the tag. This closes the
  silent type-checking hole above and, as a consequence, gives hover a type for every expression
  written inside `<div>` and friends. **This will surface type diagnostics in existing NX that
  compiles clean today.**
- **Hover answers at a member-access field**, reporting the accessed member and its type rather than
  declining. The resolver classifies the field of a `MEMBER_ACCESS_EXPRESSION` as its own context
  carrying the enclosing access's span, so the type lookup can succeed.
- **Hover answers at declaration sites that are not top-level items**: function and markup-function
  parameters, record fields, union case names, and union payload fields. Each reports its kind and
  its declared type.
- **Hover answers at a bare property value.** `<Card role=admin />` reports the union case the bare
  name resolves to, as the same name already does at a typed `let` site. The `PropertyValue` context
  stops returning nothing unconditionally, and stops reporting a zero-width span where a name is
  written.
- **Hover content becomes rich markdown in NX's own syntax.** The signature is emitted in a
  ```` ```nx ```` fenced block so the extension's existing grammar highlights it; the signature is
  spelled the way NX spells it (`let add(count:int): int`, `type Sz = int`, `let num: UserId`); and
  the kind is stated once, in TypeScript's parenthesized style for non-declaration positions
  (`(parameter) count: int`, `(property) User.name: string`).
- **Union and record hovers show their shape.** A union reports its cases and a record reports its
  fields, as component hover already reports its properties. Hovering `Role` today says nothing
  about which values exist.
- **Hover on an unannotated value reports its inferred type.** `let value = 42` reports `int` rather
  than only the kind, reading the type environment the way expression hover already does.

Explicitly out of scope:

- **Doc comments.** NX has `LINE_COMMENT` and `BLOCK_COMMENT` (`crates/nx-syntax/src/syntax_kind.rs:152`)
  but no doc-comment concept, and nothing attaches comment trivia to declarations. Adding the JSDoc
  half of a TypeScript hover is a syntax and lowering change and belongs in its own change.
- **Documentation for intrinsic elements and their attributes.** `<div>` and `class=` will hover with
  a type once inference reaches inside them, but no prose describing HTML is added.
- **Signature help, inlay hints, go-to-definition, references, rename.** The position contexts this
  change adds are what those would later read; none are added here.
- Any change to how the language service is hosted, transported, or built.

## Capabilities

### New Capabilities
<!-- None. Both halves of this change tighten requirements that already exist. -->

### Modified Capabilities
- `source-analysis-pipeline`: gains the requirement that type checking reaches every lowered
  expression in a module, including expressions written as the content or property values of an
  element whose tag resolves to no declaration. Today those subtrees are name-resolved but never
  type-checked.
- `editor-language-service`: the hover requirement gains the obligation to answer at member-access
  members, at non-top-level declaration sites (parameters, record fields, union cases, payload
  fields), at bare property values, and at expressions nested inside unresolved-tag elements; to
  report the inferred type of an unannotated value declaration; to report the cases of a union and
  the fields of a record; and to render its content as NX-fenced markdown rather than prose. The
  conservative-hover guarantee is unchanged — every position that genuinely has no metadata still
  returns nothing.
- `editor-syntax-highlighting`: gains the requirement that a hover annotation line — a parenthesized
  kind followed by the position it names, such as `(property) User.name: string` — is scoped the way
  the same names are scoped in a declaration. This is the one construct the grammar scopes that NX
  source cannot contain, and it exists because the hover fragments this change introduces for
  positions with no NX spelling would otherwise fall outside every declaration context the grammar
  scopes inside.

## Impact

- `crates/nx-types/src/infer.rs` — `infer_element_expression`'s unresolved fallthrough. The change is
  to visit children that are currently skipped, not to alter how any expression is typed.
- **Existing NX may stop compiling.** Type errors inside `<div>` subtrees are reported for the first
  time. `examples/nx/`, `sample-apps/`, and the DrawnUI catalog all nest heavily inside intrinsic
  elements and must be re-checked; any diagnostic that appears is a defect the checker was hiding,
  but the fixes are in scope for this change.
- `crates/nx-language-service/src/positions.rs` — `PositionContext` gains cases for a member-access
  member and for the non-top-level declaration sites; `name_context` stops being the catch-all that
  misclassifies them as `Reference`; `PropertyValue` carries the written name and its real span.
- `crates/nx-language-service/src/lib.rs` — `hover_contents` handles the new contexts;
  `declaration_hover`, `property_hover` and `builtin_type_hover` are replaced by a formatter that
  emits fenced NX. `Declaration::detail` stays as it is: it is completion detail, shown on one line
  in a completion list, and is a different job from a hover body. The NX spellings hover needs are
  built separately rather than by changing what completions show.
- `crates/nx-lsp/src/lib.rs` — no protocol change. `to_lsp_hover` already sends
  `MarkupKind::Markdown`; its delegation tests assert the new content.
- `src/vscode` — no code change. The fence grammar it already ships starts being exercised.
- Editor-visible in every host that consumes the service: the VS Code extension immediately, and
  drawnui-fiddle once it has a transport.
- Sequencing: `infer-unannotated-return-types` is in flight and would let function hover show an
  inferred return type. The two do not conflict; whichever lands second picks up the other's result
  with no extra work.
