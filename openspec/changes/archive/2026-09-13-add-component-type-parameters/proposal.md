## Why

A component that draws a collection through a template — a virtualized list, a table, a picker —
has to declare a property whose type depends on what the caller passes in: `itemsSource` is a list
of *something*, and the template's item parameter is that same something. NX has no way to say
"the same something" today, so the DrawnUI catalog cannot declare `SkiaLayout.ItemsSource` at all
and the templated path of every virtualized layout is unreachable from NX. The template proposal in
`examples/nx/template-candidate.nx` works around this with an `over ItemsSource` device; a small,
component-scoped form of generics removes the need for it and is what the template property will
be typed with.

Generics are a large source of complexity in the languages NX borrows from, and NX is meant to be
readable by people who are not programmers. This change deliberately takes the smallest form that
serves the UI case: a type parameter is declared with the same `name:type` syntax as any other
property, is supplied at a use site with the same `name=value` syntax as any other property, and is
never a type anyone has to write anywhere else.

## What Changes

- A component signature MAY declare **type parameters**: properties whose declared type is the
  keyword `type`, written `TItem:type`. They must come first in the property list, after
  `extends`, and are rejected on records, actions, emitted actions, state groups, and function
  parameters.
- Inside the declaring component — the rest of its signature and its body — a type parameter is a
  type: `itemsSource:TItem[]?` and `let first:TItem = ...` both work. It is a rigid nominal type
  distinct from every other type, identified by the declaration that introduced it.
- At a use site a type parameter is supplied as a **bare type name**, `<SkiaLayout TItem=Contact
  ... />`, resolved against the types visible there. Braced, quoted, and conditional forms are
  rejected at a type-parameter site.
- A type parameter that a use site does not supply is the **bottom type**, `never`. Nothing has to
  be written on a `<SkiaLayout>` that binds only children, and a non-empty `itemsSource` on one that
  omits `TItem` is a diagnostic that names `TItem` and shows the `TItem=...` spelling, rather than a
  mismatch against `never`.
- Type parameters are **not props**. They carry no value, have no default, cannot be `content`,
  do not appear in the runtime record, the property union, the IR prop schema, the generated C#
  contract record, or the serializable TypeScript `Element` type. Type-argument bindings are
  removed from an element after type checking, so nothing below the checker sees them; the
  resolved arguments are kept in the analysis result so a later change can carry them into the IR
  without reworking the checker.
- A derived component **inherits** its abstract base's type parameters open, in declaration order
  ahead of its own, and may not redeclare one.
- An instantiated component is **not a spellable type**. The element `<SkiaLayout TItem=Contact />`
  still has type `SkiaLayout`; the argument governs only the property checks at that element.
- **Inference is not part of this change.** A use site either names the argument or gets `never`.
  Inferring `TItem` from `itemsSource` is a later, strictly relaxing change: an explicit argument
  always wins, so nothing written under this change changes meaning when inference lands.
- Generated surfaces that receive values dynamically **erase** a type parameter to the host's top
  type: the IR prop schema and the C# contract record use `object`, and the TypeScript `Element`
  type uses `unknown`. Generated TypeScript surfaces that a caller invokes statically — the
  executable `Props` type and factory, and the typegen contract type — carry the parameter as a
  **generic parameter defaulting to `unknown`**, so a TypeScript caller gets the same check an NX
  author gets and pays nothing at runtime. State, and everything derived from it — the executable
  `State` type, the `T.Update` record, and the typegen state and update contracts — erases in
  every target, because no host names that instantiation.
- An emitted action declared inline in a signature is an action record of its own, checked and
  generated outside the component, so a payload field typed by a type parameter is **rejected**
  with a diagnostic rather than erased.
- The executable TypeScript resolver now resolves an optional nullable prop whose key is present
  with `undefined` to `null`, as it already did for an absent key. This is a fix to a latent defect
  that making `Props` generic surfaced, and it applies to every component, generic or not.

Out of scope, and staying out: type parameters on records, unions, aliases, element functions, and
functions; `where` constraints and equality refinements such as `Box where T=Contact`; default
type arguments; fixing a base's type parameter in a derived signature; type-argument inference.

## Capabilities

### New Capabilities

- `component-type-parameters`: the declaration form, its scope and nominal identity, the use-site
  argument form and its resolution, the `never` fallback and its diagnostics, exclusion from every
  value-level surface, inheritance of open parameters, and erasure at the host boundary.

### Modified Capabilities

- `component-syntax`: a component signature accepts `name:type` property definitions as type
  parameters and requires them to lead the property list; the same spelling is rejected in every
  other property list.
- `component-contract-inheritance`: the effective contract of a derived component includes the base
  chain's type parameters ahead of its own, and a duplicate is rejected like a duplicate prop.
- `unbraced-literal-forms`: the closed set a bare name resolves against gains one member: at a
  type-parameter site it is the set of visible type names rather than the cases of a union, and the
  unresolvable-name diagnostic says so.
- `primitive-type-names`: the bottom type gains a second source — an unspecified type parameter —
  and a diagnostic reporting a type that such a parameter fixed names the parameter rather than
  `never`.
- `nx-ir-format`: component prop schemas carry erased types and descriptors carry no type-argument
  bindings.
- `external-components`: the generated TypeScript `Props` type and factory carry a type parameter
  as a generic parameter defaulting to `unknown`; the `Element` type erases it.
- `cli-code-generation`: the generated C# contract for an exported component erases a type
  parameter to `object`; the generated TypeScript contract carries it as a generic parameter
  defaulting to `unknown`.

## Impact

- `crates/nx-syntax/grammar.js` and the generated `parser.c` — the `type` keyword becomes a valid
  property type inside a component signature; `validation.rs` enforces leading position, no
  default, no modifier, no suffix, and rejects the form outside component signatures.
- `crates/nx-hir` — `Component` gains a type-parameter list distinct from `props`; `lower.rs`
  splits them out; `components.rs` includes inherited type parameters in the effective contract.
- `crates/nx-types/src/infer.rs` — a rigid nominal type per declared parameter in scope while the
  declaration is checked; at a use site, type-argument bindings are separated from value bindings,
  resolved as type names, substituted into the effective prop types with `never` for the rest, and
  recorded for removal. `check.rs` applies that removal alongside the contextual-name rewrite.
- `crates/nx-codegen` and `crates/nx-cli/src/typegen` — the models carry the component's type
  parameters; the IR builder, the C# emitter, and the TypeScript `Element` emission erase a
  parameter reference to the host top type, and the TypeScript `Props`, factory, and contract
  emission render it as a generic parameter.
- `crates/nx-interpreter` — no change expected; it never sees a type-argument binding. Verified by
  a test rather than assumed.
- `crates/nx-language-service` — must not regress on generic components; offering type parameters
  as completions in type position is a follow-up.
- Docs — the component declaration reference gains a section on type parameters.
