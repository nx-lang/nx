## Why

NX cannot describe a UI template: a recipe for a row that a host draws once per item it decides
to show. A virtualized list needs exactly that, and the DrawnUI catalog omits `ItemsSource` and
`ItemTemplate` because NX has no spelling for either the host-owned collection's element type or
for a function. Everything DrawnUI builds on them — `RecyclingTemplate`, `MeasureItemsStrategy`,
`VirtualisationInflated` — is in the catalog and inert, and the playground's `cells` and
`uneven-cells` examples are cut down to a handful of literal rows.

A template is a function, and NX already has element functions: `let <ContactRow Item:Contact
Index:int /> = <SkiaLayout ...>` defines, invokes and expands today. What is missing is the type
to declare a property at and the ability to name a function as a value. Adding those two things
makes templates fall out of what the language already has, with no template mechanism, no slot
syntax and no new kind of declaration. `examples/nx/template-candidate.nx` explored the design;
this change lands it.

## What Changes

- **Function types.** A type reference may be a function type, spelled as an element function
  definition with `let`, the name and the body removed and `function` in the name slot:
  `<function Item:Contact Index:int />: DrawnNode`. Parameters are a property list — named,
  `Name:Type`, `content` allowed, defaults rejected — and the result type follows `/>`. `function`
  is a contextual keyword recognized only after `<` in type position. Aliases work as for any
  type: `type ContactRowTemplate = <function Item:Contact Index:int />: DrawnNode`.
- **Parenthesized types.** `( Type )` is a type reference, so a nullable or list-of function type
  is writable: `ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`. A suffix after a
  function type's result binds to the result, so parentheses are the only way to make the
  function itself nullable. (A planned optional-property syntax, `ItemTemplate?: ...`, will take
  the common case; parentheses stay for the rest.)
- **Function compatibility is by parameter name.** A function satisfies a function type when
  every parameter it declares is one the type declares, at a compatible (contravariant) type, and
  its result is compatible (covariant) with the type's. A function may declare *fewer* parameters
  than the type: the caller supplies every parameter of the type and the function ignores the
  rest. That is how a template opts out of `Index` without a default, and it makes the existing
  gap where element functions ignore parameter defaults moot for templates.
- **Functions are values.** A bare identifier that names a visible function — element or paren
  style, local, peer or imported — is a value of that function's type: `ItemTemplate={ContactRow}`
  and `Row={ContactRow}` bind it, an authored component forwards it, and a function-typed
  property or binding is invocable as an element, `<Row Item={c} Index={i} />`, with arguments
  by name. (A positional call, `Row(c, i)`, is rejected with a diagnostic: positions would depend
  on the value's own parameter order, which the subset rule leaves unknown to the caller.) There
  are no lambdas and no closures: a function is a module-level declaration, so a function value is
  a reference and captures nothing.
- **Type parameters compose with function types.** A use-site type argument (`TItem=Contact`) is
  substituted through a function-typed prop, so `ItemTemplate={ContactRow}` is checked at
  `<function Item:Contact Index:int />: DrawnNode`, and erasure below the type checker rewrites a
  parameter inside a function type to `object` as it does anywhere else.
- **NX IR carries function types and function values.** The type table gains a function kind
  (named parameters and a result), and a `reference` node that names a function declaration is a
  value in any expression position, not only as a call's callee. A module that uses one lists a
  new required feature, `function-values-v1`. Both runtimes render a function value as a record
  `{ $type: "Function", module, name }`, and the TypeScript runtime gains `callFunction`, which
  invokes such a record with named arguments — dropping arguments the function does not declare,
  as the compatibility rule allows. The conformance corpus gains a program that passes and calls
  function values.
- **The DrawnUI catalog declares templated controls.** A control with `ItemsSource: readonly
  unknown[]` and `ItemTemplate: () => SkiaControl` gains a type parameter `TItem`,
  `ItemsSource: TItem[]?` and `ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`;
  the `Item`/`Index` names are a documented divergence from DrawnUI's `BindingContext` and
  `ContextIndex`. `ItemTemplate` leaves the omitted list; `ProcessJson` and `BindingContext`
  stay.
- **The playground draws templates.** A `Function` record bound to `ItemTemplate` becomes the
  DrawnUI cell factory: each cell is a host control that, when DrawnUI binds it to an item,
  calls the function with `Item` and `Index` and draws the result inside itself, so virtualization
  and recycling are DrawnUI's own. `cells` and `uneven-cells` are ported with the virtualization
  the originals demonstrate — 100 000 items, recycling, the originals' measuring strategies — and
  stop being reduced; the examples' capability vocabulary no longer lists list virtualization as
  something NX lacks. (The originals' jump toolbar scrolls by index from code-behind, which is a
  separate gap; if it stays out, the examples are static, not reduced.)
- **Editors and docs.** Highlighting queries and the TextMate grammar scope the `function`
  keyword, parameters and result of a function type; hover renders a function type in NX spelling
  rather than `(int) => string`; `nx-grammar.md`, `nx-grammar-spec.md`, the language reference
  (including the stale `(User) => Element` section of `types.md`) and `docs/CATALOG.md` are
  updated; `examples/nx/template-candidate.nx` is replaced by a real `examples/nx/templates.nx`.

Out of scope: lambdas or anonymous templates in attribute position; inferring a type argument
from a sibling binding (`over ItemsSource` in the candidate file — `TItem=Contact` at the use site
is what `component-type-parameters` specifies); optional-property syntax (`name?:`), which is its
own change; parameter defaults on element functions; action handlers bound inside a template cell,
which the playground draws inert; the DrawnUI fiddle, which follows the published packages.

## Capabilities

### New Capabilities

- `function-types`: the function type reference — its spelling, parameter rules, the result
  type, the contextual `function` keyword, aliasing, its display — and compatibility between
  functions and function types by parameter name, with the subset rule.
- `function-values`: a function name as a value, its type, binding it to function-typed
  properties and values, invoking a function-typed value as an element, and the
  canonical `Function` record both runtimes render.

### Modified Capabilities

- `type-reference-suffixes`: a parenthesized type is a base type that suffixes apply to.
- `component-type-parameters`: a type argument is substituted through a function-typed prop, and
  erasure rewrites parameters inside function types.
- `nx-ir-format`: the function type kind, function references as values with the
  `function-values-v1` feature, and a corpus program covering them.
- `typescript-ir-runtime`: the canonical `Function` record, `callFunction` with named arguments,
  and function-valued props on descriptors.
- `drawnui-nx-catalog`: templated controls are declared with a type parameter, `ItemsSource` and
  `ItemTemplate`; the exclusion of function-typed members narrows to those that are neither
  events nor item templates.
- `playground`: `ItemTemplate` functions draw virtualized cells, the two list examples are
  complete, and the capability vocabulary drops list virtualization from what NX lacks.
- `editor-syntax-highlighting`: a function type inside a property list or alias is scoped.
- `editor-language-service`: hover renders function types in NX spelling.
- `dotnet-binding`: a function value is read through `NxFunctionRef`, and generated C# types a
  function-typed member with it rather than a delegate, which neither output format can serialize.

## Impact

- `crates/nx-syntax`: `grammar.js` (`function_type`, `parenthesized_type`, the `function`
  literal), `syntax_kind.rs`, `validation.rs` (suffix walk over the new base kinds, defaults
  rejected in function types), `queries/highlights.scm`, fixtures and snapshots.
- `crates/nx-hir`: `ast::TypeRef::Function` gains parameter names; `lower_type` lowers both new
  kinds; `erase_type_parameters` recurses into function types; the prepared namespace exposes
  function definitions as value bindings.
- `crates/nx-types`: `Type::Function` gains parameter names; `is_compatible_with` matches by
  name with the subset rule; `infer_call` and `infer_element_expression` accept a function-typed
  value as callee; `Expr::Ident` naming a function yields its type; display in NX spelling.
- `crates/nx-interpreter`: `Value::Function`, identifier evaluation, element and call
  dispatch on a function value with the subset rule, serialization; `crates/nx-api` renders and
  refuses to decode the `Function` record as it does `ActionHandler`.
- `crates/nx-codegen`: type kind, reference-as-value in `builder.rs`, the feature, `ir_explain`,
  image validation; `specs/ir-conformance` gains a program; `.NET`/FFI/wasm/Node pass images
  through unchanged.
- `crates/nx-cli/src/typegen`: a function-typed prop maps to a TypeScript function type, and to
  `NxFunctionRef` in C#; `bindings/dotnet` gains that DTO beside `NxActionHandlerRef`.
- `crates/nx-language-service`: hover text for function types.
- `runtime/typescript/src/index.ts`: reader validation for the type kind and feature, canonical
  rendering of function references, `callFunction`, descriptor props; tests and the corpus run.
- `sites/playground`: `scripts/generate-catalog.mjs`, regenerated `catalog/*`, a template cell
  control and factory in `src/render`, `coerceProps`, `cells.nx`, `uneven-cells.nx`,
  `examples.json`, `types.ts`, `docs/CATALOG.md`, `README.md`.
- `src/vscode/syntaxes/*.tmLanguage.json`, `nx-grammar.md`, `nx-grammar-spec.md`,
  `docs/src/content/docs/reference/syntax/{types,functions}.md`, `examples/nx/templates.nx`.
