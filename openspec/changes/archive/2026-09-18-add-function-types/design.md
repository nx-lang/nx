## Context

See proposal.md for motivation. What shapes the approach is how much of a function type already
exists, in pieces that were never connected:

- The parser's `type` rule is `base suffix*` with two bases (`crates/nx-syntax/grammar.js:226`);
  `<` cannot start a type, and `(` cannot either, so both new bases are free.
- `ast::TypeRef::Function` and `Type::Function` exist (`crates/nx-hir/src/ast/types.rs:30`,
  `crates/nx-types/src/ty.rs:210`) with **positional** parameters; `lower_type` never builds one,
  and `is_compatible_with` zips parameters by position (`ty.rs:526-552`).
- The checker already binds every visible function into its environment as a `Type::Function`
  (`crates/nx-types/src/infer.rs:4550-4606`), which is why `ItemTemplate={ContactRow}` is
  silently accepted today: `Expr::Ident` finds it. Functions live in the prepared `Element`
  namespace (`crates/nx-hir/src/prepared.rs:34`), which is why the interpreter and codegen do not.
- `nx-codegen` maps `CodegenTypeRef::Function` to the top type with a comment saying no syntax
  reaches it (`crates/nx-codegen/src/ir.rs:1077`); a `reference` node (kind 5) already names a
  function declaration as a call's callee, and the TypeScript runtime already evaluates it to an
  internal `functionReference` (`runtime/typescript/src/index.ts:2106`).
- The Rust interpreter has no function value (`crates/nx-interpreter/src/value.rs:28-129`);
  `ActionHandler` is the precedent for a runtime-only value with a public rendering.
- Component type parameters are explicit at use sites (`TItem=Contact`) and erased to `object`
  below the checker; `erase_type_parameters` passes `TypeRef::Function` through untouched
  (`crates/nx-hir/src/components.rs:1099`).
- DrawnUI's vendored TypeScript port exposes `ItemsSource`, `ItemTemplate: () => SkiaControl`,
  `BindingContext` and `ContextIndex` as live setters (`sites/playground/src/drawnui/controls/
  SkiaLayout.ts:185-206`, `core/SkiaControl.ts:139-148`); cells are created by DrawnUI's own
  factory outside React, so a cell cannot be drawn through the playground's React reconciler.
- IR schema 4 is unreleased on this branch, so its tables can be amended in place; the feature
  list is still the mechanism by which an older runtime refuses a module by name.

## Goals / Non-Goals

**Goals:**
- One function type in the type system, with named parameters, that both definition styles
  inhabit and that templates, aliases and props all use.
- A function value that is a plain reference — no environment, no capture — so every runtime
  and the IR can carry it as a name.
- Templates reach DrawnUI's own virtualization; the playground adds no list machinery of its own.

**Non-Goals:**
- Positional invocation of a function-typed value (needs the call site to carry the static
  type's parameter order into the runtimes; deferred until a call site is annotated with it).
- A Rust host API for invoking a `Function` record from outside NX (`nx-api` renders it and
  refuses to decode it, as with `ActionHandler`); the TypeScript runtime is the only host that
  needs it now.
- Changing how element-function parameter defaults behave.

## Decisions

### D1. Named parameters replace positional ones in `TypeRef::Function` and `Type::Function`

Both variants gain `params: Vec<FunctionParam { name, ty, is_content }>` and keep `ret`. Every
producer is a declaration (`bind_function_signature_from_parts`, imported interface signatures)
and already has the names; every existing consumer (`infer_call`, `substitute_parameters`,
`find_parameter`, display) keeps working by reading `param.ty` in order. Compatibility
(`is_compatible_with`) becomes: for each param of the *value's* type, find the same name in the
*expected* type, check expected → value contravariantly; content must pair with content; return
covariant; any value param absent from the expected type fails. This is the subset rule from the
spec, and it is one function, not a second type.

*Alternative rejected:* a separate "element function type" beside a positional one. Two types
for one concept, and the split would leak into the IR and both runtimes.

### D2. Grammar: `function_type` and `parenthesized_type` as bases of `type`

```
type              := (primitive_type | user_defined_type | function_type | parenthesized_type) suffix*
function_type     := '<' 'function' property_definition* '/' '>' ':' type
parenthesized_type:= '(' type ')'
```

`'function'` is a string literal in the rule; tree-sitter's lexer only offers it in states where
it is valid, so `function` stays an identifier everywhere else with no keyword reservation.
Parameters reuse `property_definition` so `content` and the `name:type` shape come free; the
validator rejects a default, a `type`-typed parameter (extend the existing site table in
`validate_type_parameter_definition` with a `FUNCTION_TYPE` arm) and a second `content`.
`validate_type_suffix_chain` skips the first child as the base — unchanged — and gains one rule:
when the base is `parenthesized_type` whose inner type ends in `?`, a `?` immediately after `)`
is the duplicate-nullable error. The trailing `type` of a `function_type` greedily takes suffixes,
which is what the spec wants (suffix binds to the result).

New syntax kinds: `FUNCTION_TYPE`, `PARENTHESIZED_TYPE`. `lower_type` gains both arms;
`PARENTHESIZED_TYPE` lowers to its inner type (no HIR node — it is purely syntactic).

*Alternative rejected:* result type inside the tag (`returns DrawnNode`). Solves the suffix
ambiguity without parentheses but breaks the mirror with `let <F ... />: R = ...`; with optional
properties coming, parentheses are needed rarely enough that the mirror is worth more.

### D3. Functions become visible in the value namespace; a function value is a reference

`PreparedItemKind::Function` reports both `Element` and `Value` namespaces, so a bare identifier
naming a function resolves in every consumer through the same binding table the checker already
uses. Lexical scopes still layer over top-level bindings, so a parameter or prop named `Row`
shadows a function `Row`.

- **Checker:** `Expr::Ident` already yields the function's `Type::Function`; nothing to add
  beyond the named-param shape.
- **Interpreter:** `Value::Function { module: ModuleId, name: SmolStr }`. `try_lookup_variable`
  miss → `resolve_item(module, name)` for `Item::Function` → the value. Rendered by `nx-api` as
  `NxValue::Record { type_name: "Function", properties: { module, name } }`; `from_nx_value`
  refuses it like `ActionHandler`. Equality: same module and name.
- **Codegen:** the `Expr::Ident` path in `builder.rs` (which today errors with
  `codegen-unresolved-name`) emits a `reference` node to the function declaration, the same node
  `build_function_element_call` uses as a callee. A `reference` to a function anywhere but a
  callee, or a function type in the type table, adds `function-values-v1` to the required
  features.
- **TypeScript runtime:** `functionReference` internal values already exist; canonicalization
  renders them as `{ $type: "Function", module, name }` where `module` is the linked module's
  identity (the string the corpus keys results by, e.g. `app/main.nx`). Decoding a host-supplied
  `Function` record at a function-typed prop resolves `module`/`name` against the linked program
  and fails by name otherwise.

No closures: a component body has no nested `let` and functions are module-level, so a reference
is the whole value. This is a property of the grammar, and the spec states it so nothing later
quietly adds capture.

### D4. Invoking a function-typed value: element form only, a `namedCall` IR node

In `infer_element_expression`, before resolving the tag as a declared element, look the tag up in
the lexical environment; if the binding's type is `Type::Function`, check the bindings against
that type's parameters (all required, content to content, unknown rejected) and return `ret`.
Otherwise fall through to today's resolution — so a `string` prop named `Label` does not hijack
`<Label />`. A paren call whose callee is a lexical function-typed binding is rejected by
`infer_call` with a diagnostic showing the element form.

The interpreter mirrors it: in `eval_element_expr`, a tag that names a scope variable holding
`Value::Function` becomes a call — resolve the declaration, bind the evaluated fields by name,
drop names the declaration lacks, error on a declared parameter that is missing.

The IR needs a node that carries argument *names*, because the callee is a slot whose declaration
the runtime learns only at run time. Node kind 21, `namedCall`: `[21, callee, count, name₀, arg₀,
…]`. The existing `call` node (positional, callee statically a function reference) is unchanged.
`namedCall` is gated by the same `function-values-v1` feature. The TypeScript runtime evaluates
`callee`, requires a `functionReference`, and binds by name with the subset rule; `callFunction`
is the same binding exposed to hosts with a canonical `Function` record as the callee.

*Alternative rejected:* reuse `call` with args in the static type's order. The runtime cannot
map positions to the callee's own parameters without the static type; carrying names is simpler
than carrying types.

### D5. Type parameters: substitute through, erase through

`substitute_parameters` already recurses into `Type::Function`; with D1 it recurses into
`param.ty`. `check_element_bindings_against_component` substitutes prop types before checking,
so `ItemTemplate={ContactRow}` is checked at `<function Item:Contact Index:int />: DrawnNode`
with no new code path. An unbound parameter is substituted by variance
(`Type::substitute_parameters_by_variance`): the bottom type in a covariant position, as before,
and the top type `object` in a contravariant one — a function type's parameter. Bottom there would
make every template pass vacuously (`never` satisfies `Contact`), while `object` makes a template
that assumes `Contact` fail, and the existing "TItem was not specified" wording is applied to that
diagnostic. (The spec scenario's `apply` is an NX intrinsic name; the scenario uses `invoke`.) `erase_type_parameters` gains recursion into
`TypeRef::Function` so codegen and typegen see `object`.

### D6. IR type kind `FUNCTION = 4`

Entry: `[4, result_type, param_count, name₀, type₀, flags₀, …]`, `flags` bit 0 = content.
Interned like every type. `ir_explain` prints NX spelling. The Rust and TypeScript readers add
the kind to their validation tables (result and each type index in range, each name index in
range). `CodegenTypeRef::Function` gains names; the "nothing reaches here" branch is deleted.
Typegen (`crates/nx-cli/src/typegen`) maps a function type to a TypeScript function type with
named parameters, `(args: { Item: Contact; Index: number }) => DrawnNode`, since NX arguments
are by name.

### D7. Catalog: templated controls from the `ItemsSource`/`ItemTemplate` pair

`generate-catalog.mjs` detects a class declaring both members. It emits `TItem:type` as the
first property (the validator requires type parameters first), `ItemsSource:TItem[]?`,
`ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)?`, removes both from
`omitted.json`, and records `templates: { ItemTemplate: ["Item", "Index"] }` in
`catalog-meta.json` beside `events`. The pair rule is class-based, so any control that declares
both (SkiaLayout today) gets it. `docs/CATALOG.md` records the `Item`/`Index` naming as a
divergence from `BindingContext`/`ContextIndex`.

### D8. Playground: a template cell control drawn outside React

`coerceProps` treats a `Function` record on a prop the metadata lists under `templates` as a
factory: `() => new NxTemplateCell(program, record, report)`. `NxTemplateCell` extends
`SkiaLayout` (Absolute, `HorizontalOptions=Fill`) and overrides `OnBindingContextChanged` to:
`callFunction(program, record, { Item: BindingContext, Index: ContextIndex })`, clear its
children, materialize the result, invalidate measure. Materialization is a small function in
`src/render` that reuses the reconciler's `Registry`/`ApplyInitialStyles`/`applyProps` (exported
from `reconciler.ts`) and `coerceProps`/`childrenOf`, recursing into children; an authored
component descriptor is initialized with `initializeComponent` without a parent and its rendered
output materialized; handler records inside a cell are skipped and reported once as inert.
`ItemsSource` passes through `coerceProps` uncoerced, so the item a cell receives is the
canonical value the function expects. Failures are reported through the same channel as dispatch
failures (`useNxDrawing`), tagged with the index, and the cell is left empty.

Recycling: DrawnUI rebinds an existing cell, which re-runs the override; rebuilding the cell's
children per bind is acceptable because cells cache as images (`UseCache=Image`) and the
original demo does the same amount of work per bind in C#.

### D9. Examples

`cells.nx` and `uneven-cells.nx` define their cells as element functions
(`let <ContactCell Item:Contact Index:int /> = ...`), build their collections with nested `for`
over a digit list (NX has no range), and bind `TItem=Contact ItemsSource={contacts}
ItemTemplate={ContactCell}` with the originals' `RecyclingTemplate` and `MeasureItemsStrategy`.
`examples/nx/templates.nx` replaces `template-candidate.nx` as the language-level example, with
`DataTable`-style multi-template props and a forwarding component.

## Implementation notes

Recorded while implementing, where the code departed from or refined the decisions above.

- **No range, no flattening.** NX has no range expression, and a nested `for` yields a nested list
  (`Contact[][]`) rather than splicing, in a braced list and in element content alike. What does
  splice is a *list-valued child* of an element: `<Contacts> <Shift Items={seed} By=0 />
  <Shift Items={seed} By=10 /> </Contacts>` concatenates the two lists into the content
  parameter. The examples build their collections with that idiom — a seed of ten, shifted ten
  times per step through an element function with a `content` parameter — which reaches 100 000
  items in five steps. D9's "nested `for` over a digit list" was wrong about the flattening.
- **The TypeScript runtime did not splice list-valued content children** where the interpreter and
  the checker did (two `for` loops side by side, or list-returning calls, among a component's or
  a function call's children). It does now (`contentAt`, and the content parameter of
  `invokeFunction`), pinned by an emitted-IR test against the native interpreter.
- **The interpreter did not erase component type parameters** when coercing props and state, so
  `ItemsSource={contacts}` with `TItem=Contact` failed at run time although the checker had
  consumed the argument. `normalize_component_props` and `component_state_fields` now erase them
  to `object`, as every surface below the checker does.
- **A function's parameters shadowed its own name.** `infer_function` bound parameters in the
  global scope and removed them afterwards, which removed a same-named top-level binding too. They
  now get a scope of their own; the `conversions` corpus result for `joinedList` changed from
  `[3, 1.5]` to `[3.0, 1.5]` because the widened join of the top-level `count` is now recorded.
- **Evaluation time.** The 100 000-item collection compiles in 24 ms and evaluates in 392 ms under
  Node's V8 (the browser's engine); the cells example keeps the original's count. Uneven cells is
  the original's initial 200 posts, since `LoadMore` pages from code-behind.
- **The playground's tests load DrawnUI under Node** through `scripts/node-url-imports.mjs`, which
  resolves Vite's `?url` imports and extensionless relative imports, plus
  `--experimental-transform-types` for the port's parameter properties; the reconciler's registry
  and `applyProps` moved to `src/drawnui/react/registry.ts` so the materializer needs no React.
- **`apply` is an NX intrinsic**, so the `function-values` scenario's paren function is `invoke`.

- **The C# contract carries `NxFunctionRef`, not a delegate.** The TypeScript backend spells a
  function type as a function of one object of named arguments, which is how NX binds arguments.
  C# has no such shape, and the `System.Delegate` the backend fell back to serialized in neither
  supported format: MessagePack has no formatter for it — and resolves formatters per *member* of
  the containing type, so declaring one function-typed prop made the whole contract, its `_update`
  record and its property companions unserializable even with the member null — while
  `System.Text.Json` refuses a delegate on read and write alike. A .NET host cannot call a
  function value anyway (no FFI `callFunction`, and `from_nx_value` refuses a `Function` record),
  but it should be able to *read* which template it was handed, so the mapping follows the
  `ActionHandler` precedent the rest of this design invokes: `NxLang.Nx.NxFunctionRef`
  (`[MessagePackObject]`, `module` + `name`) is the rendered `Function` record, as
  `NxActionHandlerRef` is the rendered `ActionHandler` record. `NxFunctionValueTests` in
  `bindings/dotnet` round-trips it through both formats, including with the member null.
- **One spelling of a function type, assembled in one place.** Diagnostics, hovers and the
  explained form of an IR artifact read three different representations, so each renders the parts
  itself and `nx_hir::ast::spell_function_type` assembles them. `spell_type_ref` is shared the same
  way, so the language service no longer keeps a second copy. Two tests compare the assembled
  spelling with the checker's, from the hover and from the explained artifact.

## Risks / Trade-offs

- [A 100 000-record list is evaluated eagerly by the IR runtime before DrawnUI sees it] → Measure
  first; if evaluation or canonicalization is visibly slow in the browser, the examples use
  10 000 items and the note says so. The virtualization claim is about cells, not items.
- [Rebuilding cell children on every rebind may stutter on fast scrolls] → Cells are image-cached;
  if profiling shows it, a later change can diff the materialized tree against the new value.
- [Making functions visible in the value namespace could collide with a same-named `let` value]
  → The prepared-module builder reports a function and a value of one name as a duplicate
  declaration once, rather than letting the two namespaces disagree about which wins; a fixture
  pins the diagnostic.
- [`function` as a contextual token could confuse the error recovery of a malformed type] →
  Fixture tests for `Name: <functon ... />` and `Name: <function` without `/>` pin the diagnostics.
- [The bottom-type fallback for an unbound `TItem` makes every template mismatch until
  `TItem=` is written] → This is the same rule `ItemsSource={contacts}` already follows; the
  diagnostic names the parameter and shows `TItem=`, and the catalog docs show the form.
- [Schema 4 amended in place] → It is unreleased; the feature gate still lets a runtime that
  predates this change refuse a module by name. The corpus images that gain a function type or
  reference are regenerated and the explained-text diff is reviewed.
