## Why

A nominal type in NX is identified by its spelling, resolved in whatever scope is asking. Two
unrelated types that share a name are therefore the same type, and a type that the asking module
cannot name does not resolve at all. Both failure modes are live today:

```nx
// widgets.nx
export type Fit = fill | contain | cover
export let <Img fit: Fit = {Fit.fill} /> = <div class="img" />

// app.nx
import { Img } from "./widgets.nx"
type Fit = fill | contain | cover      // a different Fit, identically shaped

let root() = { <Img fit={Fit.stretch} /> }
```

This compiles clean, and the local `Fit` stands in for the declaring module's. `contextual-literal-
binding` closed the *differently shaped* half of this at the type-checking layer by resolving a
declaration's property types in the module that declares them, which makes the expected type a
resolved `Type::Union` carrying its cases rather than a `Type::Named` that matches on spelling.
`replace-enums-with-unions` then extended that to unions and made a case satisfy a union only when
the union declares it. Both fixes stop at type checking, because everything below still reaches a
type by name.

Both also stop short of identity. `UnionType` compares by `{name, cases, base}` — case *names* only —
so the hole closes where two declarations differ in their case names and stays open where they do
not. Give the local `Fit` the same cases, as above, and it is accepted again, silently. The same
holds one level down, where the case names match and a payload field's type does not:

```nx
// widgets.nx
export type S = idle | busy { n: int }
export let <Draw s: S = {<S.idle />} /> = <div class="d" />

// app.nx
import { Draw } from "./widgets.nx"
type S = idle | spinning             // a different S, and `spinning` is not a case of theirs

let root() = { <Draw s=spinning /> }   // accepted today, with no diagnostic
```

Both are RF1 and RF6 in `contextual-literal-binding`'s `review.md`, and neither is fixable by
comparing shapes harder: two declarations can agree on every observable and still be different types.
That is what origin supplies.

The consequence is that a bare contextual name resolves correctly and then cannot be lowered. A
case of a union the using module never imported has no spelling that survives: the rewrite emits an
identifier resolved by visible name, and under a wildcard alias the qualified form does not lower
either — `ui.Fit.cover` reports *"Member access not yet implemented"*. So the headline promise of
contextual literals, writing `<Img fit=cover />` against a component from a UI library without
importing `Fit`, is specified but unreachable. It reports the needed import instead.

## What Changes

- Nominal types — unions, union cases, records, and components — carry a canonical declaring origin,
  the `(module identity, definition id)` pair that already addresses items in a resolved program,
  alongside the name used to display them. Type identity becomes origin equality rather than name
  equality, and record and component subtyping compares declarations rather than spellings.
- A declaration's type references resolve in the namespace of the module that wrote them — which
  includes what that module imported — so a consumer no longer re-resolves a foreign contract in its
  own namespace. This applies one link at a time along a record's or component's inheritance chain,
  so `extends` resolves where it was written too. (The original plan was to publish resolved
  identity in the interface; see design D3 (revised) for why the import graph already answers this.)
- A resolved contextual name lowers to a reference that carries its origin, so a union case reaches
  code generation and the interpreter without the declaring type being
  nameable at the use site. This removes the import guidance that
  `contextual-literal-binding` currently reports in that position.
- Language-service completions and lookups resolve elements and property types through the import
  graph rather than a flat list of every workspace declaration, so aliases and module identity are
  preserved and declarations invisible to the current module are not offered.
- A value supplied by an embedder is checked against the declaration that expects it. Its type name
  is a label read in the namespace of the module that declared the property or emitted action, not
  in the module that bound the handler — an embedder cannot supply an origin, so the expected side is
  where identity has to come from.
- Diagnostics disambiguate two same-named nominal types, replacing the current
  `expects Fit, found Fit`.

## Capabilities

### New Capabilities

- `nominal-type-identity`: what identifies a union, union case, record, or component across module
  boundaries; how origin is carried through interfaces, lowering, code generation, and runtime
  values; how a host-supplied type name is read when it cannot carry an origin; and how identity
  governs type equality, subtyping, and diagnostics.

### Modified Capabilities

- `symbol-resolution-model`: resolved types and lowered references carry canonical definition
  identity, so lookup by visible name is no longer the only way to reach a definition.
- `enum-values`: a bare case whose union is not nameable in the using module lowers rather than
  reporting that the union must be imported; equality of two same-named unions is decided by origin.
- `discriminated-unions`: a payloadless case of a foreign union is constructible without importing
  the union, including cases that inherit fields from an abstract base; a union case value is
  distinguishable from a record at runtime.
- `editor-language-service`: completions resolve through the import graph, preserving aliases and
  module identity.

## Impact

- `crates/nx-types`: `Type`, `NamedType`, `UnionType`, `UnionCaseType`, equality and compatibility,
  the contextual-name resolution path, and the lookups that run *after* a type resolves — a union's
  definition, its base, and its members are reached by the declaration the resolved type names
  rather than by the spelling it was reached under (design D7).
- `crates/nx-hir`: `DeclaringOrigin` and the peer namespace, prepared bindings, interface items, and
  record and component resolution including their inheritance chains.
- `crates/nx-api`: interface construction and the artifact representation of nominal types.
- `crates/nx-codegen`: reference resolution for union cases; NX IR emission.
- `crates/nx-interpreter`: union resolution by origin rather than visible name; the emit's declaring
  module on `Value::ActionHandler`; peer namespaces on the runtime prepared module; record and
  component subtyping at the host boundary decided by declaration; component entrypoints evaluating
  in the declaring module. Record and component *values*
  still carry a type name rather than an origin, by design — an embedder cannot supply one, so the
  name is read as a label in the expecting module's namespace (design D6).
- `crates/nx-language-service`: declaration lookup and completion contexts.
- Follow-on for `contextual-literal-binding`: RF1's deferred half, RF3, and RF6's remaining half in
  that change's `review.md` are resolved here. **Not** RF2 — that was the formatter's dotted-record
  heuristic, and `replace-enums-with-unions` dissolved it by making a constant case a scalar. RF5
  was fixed there too. RF6's same-name/different-case-names half also closed there, because the
  unified resolution path required it; what remains here is the payload-field-type half, which needs
  declaring origin in type identity.

## Sequencing

This change modifies requirements as `replace-enums-with-unions` leaves them — the `enum-values`
requirement *Constant cases are referenceable without naming the union type*, the
`discriminated-unions` requirement *Payloadless union cases support contextual construction*, and
the `editor-language-service` completions requirement. That change has archived, and
`openspec validate nominal-type-identity --strict` passes against the specs as it left them.

`unbraced-literal-forms` is no longer modified here: that bullet was the formatter's field-count and
dotted-name heuristic (RF2), which `replace-enums-with-unions` dissolved by making a constant case a
scalar.
