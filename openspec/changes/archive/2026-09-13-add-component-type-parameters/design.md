## Context

See proposal.md — Why. The facts that shape the approach:

- The grammar is tree-sitter (`crates/nx-syntax/grammar.js`, generated `parser.c`). `type` is
  already a keyword, used by `type_definition`, `union_definition`, and `record_definition`. A
  component signature parses its props through `_component_property_definition`, whose type field
  is the shared `type` rule: a primitive or qualified name plus `?`/`[]` suffixes. `word` is
  `identifier`, and a prop name may be a `markup_identifier`, so a prop *named* `type`
  (`type:string`) is a case to protect.
- Lowering turns every property definition into a `RecordField { name, ty: ast::TypeRef, is_content,
  default }`, and `Component.props` is a `Vec<RecordField>`. Every consumer of `props` — binding
  checks, defaults, content binding, body scope, the runtime record, the IR prop schema, generated
  `Props`, external contract records — is a value-level surface.
- `nx-hir::components` computes an `EffectiveComponentContract` for a component by walking its
  abstract base chain, and is where duplicate props and duplicate content props are rejected.
- The checker (`crates/nx-types/src/infer.rs`) binds a component's effective props into the value
  environment in `infer_component`, then infers the body. At a use site,
  `check_element_bindings_against_component` builds an `ElementBindingSpec` keyed by prop name and
  `check_element_bindings` checks each property path against it. Property entries may be
  conditional (`if`, condition lists, `match`), and `property_paths_for_entries` expands them.
- A bare name at a property value is `Expr::ContextualName`, typed `Type::ContextualName` and
  resolved against the expected type at the binding site. Resolutions are recorded and, in
  `check.rs`, rewritten into the module before it is snapshotted, so nothing below the checker sees
  the bare spelling. Integer-to-float literal conversion uses the same mechanism.
- A nominal type is `Type::Named(NamedType { name, origin: Option<DeclaringOrigin> })`, and
  `DeclaringOrigin` addresses one definition in one module. Identity is by origin.
- `Type::Primitive(Primitive::Never)` exists, is the element type of an empty list, satisfies every
  type, and is rendered as `never`. `mentions_never` already tracks types an empty list fixed so
  diagnostics can spell them as `{}`.
- The interpreter rejects a prop the target component does not declare (`unknown prop`).
- `nx-codegen` (executable TypeScript and IR) and `nx-cli` typegen (C# and TypeScript contracts)
  both map a prop's `ast::TypeRef` by name, and both already map the name `object` to the host top
  type (`object` in C#, `unknown` in TypeScript).

## Goals / Non-Goals

**Goals:**

- Type parameters are a separate thing from props everywhere below the parser, so that every
  exclusion the spec requires holds by construction rather than by filtering.
- One place removes type-argument bindings from an element, and it is the same place that already
  removes other checker-only spellings.
- One erasure rule, applied wherever a target cannot carry a type parameter, and a generic
  parameter wherever a TypeScript caller names the instantiation statically.
- The checker's use-site work is shaped so that inference can later be added by changing how the
  substitution map is populated, and nothing else.

**Non-Goals:**

- No general type-application or substitution machinery beyond replacing a parameter with one
  concrete type in a component's own prop types.
- No changes to the interpreter's evaluation of components.
- No editor features beyond not regressing; completions for type parameters are a follow-up.

## Decisions

### D1: `Component` carries `type_params` as a list separate from `props`

Lowering splits a signature's `type`-typed definitions into `Component.type_params:
Vec<TypeParameter { name, span }>` and leaves `props` as it is. The effective contract gains a
matching `type_params` list, with each entry carrying the declaring component's module identity,
inherited entries first.

*Why:* every existing consumer of `props` is a value-level surface, and the spec requires type
parameters to be absent from all of them. A flag on `RecordField` would make each of those
consumers responsible for skipping flagged fields; a separate list makes their absence the default
and their presence an explicit choice in the two places that want them (the checker, and erasure).

*Alternative considered:* `RecordField.is_type_parameter`. Rejected for the reason above, and
because a `RecordField` has a `ty`, `default`, and `is_content`, none of which a type parameter has.

### D2: The grammar accepts `type` as a property type; validation decides where it is legal

`_component_property_definition` and `property_definition` both gain `'type'` as an alternative
for their type field, producing a PROPERTY_DEFINITION whose type child is the keyword token.
`validation.rs` then rejects, with a targeted message: a `type`-typed definition anywhere but a
component signature; one that follows a regular prop; one with a default; one with a modifier.
Suffixes on the keyword are impossible by grammar (`'type'` is not the `type` rule, so `?` and
`[]` do not attach), which makes `TItem:type?` a parse error.

*Why:* a parse error inside a record says "unexpected syntax" and nothing more. Accepting the
form and rejecting it in validation is the pattern the file already uses ("the targeted diagnostic
has already said what the generic one would"). Paren-style function parameters reuse `property_definition`, so
`let f(T:type)` parses and validation rejects it with the same targeted message as a record field.

*Risk handled here:* the prop-named-`type` case. Tree-sitter keyword extraction applies to
`identifier`; `markup_identifier` is a distinct token, so `type:string` in a signature currently
lexes the name as a markup identifier. A parser test pins that before the grammar changes.

### D3: A rigid type parameter is a new `Type::Parameter` variant, not a `Named` with a synthetic origin

`Type::Parameter(TypeParameterRef { name, owner: DeclaringOrigin, ordinal })`. It renders as its
name; it satisfies only itself, `object`, and expected types that are itself under `?`/`[]`; only
itself and the bottom type satisfy it; joining it with anything else is a mismatch. A
`substitute(ty, &map)` walks a `Type` replacing parameters by owner and ordinal.

*Why:* reusing `Named` needs a `DeclaringOrigin` per parameter, and an origin addresses a
definition, not a member of one; inventing definition ids for parameters would leak into every
origin consumer. A dedicated variant is a few match arms in `satisfies`, `join`, and `Display`,
and it makes erasure and the "fixed by an unspecified parameter" check a type test instead of a
name comparison.

While a component is being checked (its prop and state annotations, defaults, and body), the
checker holds a type scope mapping each effective type parameter's name to its `Type::Parameter`.
`resolve_named_type` consults that scope first, which is what gives shadowing. Inherited prop
annotations are resolved in the declaring module today; the scope is keyed by name and effective
names are unique, so an inherited `items:TItem[]` resolves to the base's parameter without
special casing.

### D4: A use site is checked by substitution into a copy of the effective prop types

In `check_element_bindings_against_component`:

1. Partition the element's property entries. A plain, unconditional `Value` entry whose key is an
   effective type parameter is a type argument; any other entry with such a key (a value inside an
   `if`, condition list, or `match` fragment) is a diagnostic. The remaining entries are value
   bindings and go through the existing path expansion unchanged.
2. Resolve each argument. Its expression must be `Expr::ContextualName`; anything else is a
   diagnostic that shows the bare form. The name resolves through the type-name resolver
   (primitives, aliases, unions, records, components, and the enclosing component's parameters),
   never the value environment. A name that reaches nothing is a diagnostic with a near-match
   suggestion drawn from the visible type names, on the same terms as the existing union-case
   suggestion.
3. Build the substitution map: every effective parameter maps to its argument, or to
   `Type::never()` when none was bound.
4. Build the `ElementBindingSpec` from the effective props with each prop type substituted, and
   record on each `ElementPropertySpec` the name of an unspecified parameter that its unsubstituted
   type mentioned, if any. Type parameters get no spec entry, so they are never "missing".
5. Record the type-argument expressions for removal (D5).

`check_property_path_bindings` gains one branch: when a binding fails against a spec entry that an
unspecified parameter fixed, it reports `Property 'itemsSource' on 'SkiaLayout' is typed by
'TItem', which was not specified; add TItem=<type>` instead of the mismatch. The satisfaction test
runs first and quietly, so a binding that succeeds against `never[]?` (an empty list, `null`)
stays silent.

*As implemented:* the substitution is applied during resolution rather than as a second walk.
The checker keeps one type-parameter scope (`name → Type`); `infer_component` fills it with the
component's rigid parameters for the length of the declaration, and a use site swaps in the
target's argument map (rigid markers for unbound parameters) while the contract's prop types are
resolved, then restores the enclosing scope before the bound values are inferred. Only the names a
type reference spells directly consult the scope, so an alias's target and a foreign declaration's
signature resolve where they were written. After resolution, each spec entry that still mentions
one of the target's rigid markers is rewritten to the bottom type and remembers the parameter's
name. The resolved arguments are kept on the analysis result as
`ModuleArtifact::element_type_arguments`, keyed by element.

*Why substitution rather than unification:* the change has no inference, so the map is fully
known before any binding is checked. When inference is added, step 3 becomes "unify the bound
values against the unsubstituted prop types to fill the gaps, explicit arguments winning", and
steps 4 and 5 do not change. That is the whole point of shaping it this way now.

### D5: Type-argument bindings are removed in `check.rs`, next to the contextual-name rewrite

The inference context collects the `ExprId` of every consumed type argument, and alongside it a
map from each element expression to its resolved arguments (parameter name to `Type`). After
`finish()`, `check.rs` calls a new `nx_hir::remove_property_entries(&mut prepared_module, &ids)`
in the same block that applies contextual-name resolutions and int-literal conversions, and
returns the resolved-argument map with the type environment as part of the analysis result. Below
that point no element carries a type argument, so the interpreter, IR builder, and executable
emitter are unchanged and their existing "unknown prop" handling is what a type argument would hit
if one ever leaked. Nothing in this change reads the resolved-argument map; it exists so that
carrying use-site arguments into the IR later is an additive field on the descriptor rather than a
change to how the checker consumes them.

*Why one place:* this is the established pattern for source spellings that only the checker
understands, and it keeps the interpreter out of the change. A consequence, accepted: evaluating
unchecked HIR that binds a type argument (as the interpreter's direct-HIR tests do for other
constructs) reports `unknown prop`, which is correct for unchecked input.

### D6: Erase where the host receives values dynamically; go generic where a TypeScript caller names the instantiation

A type argument is a static fact at an NX use site and never reaches the wire, so a target that
receives a component value by `$type` at runtime has nothing to bind a parameter to. Those targets
erase: the IR prop schema, the C# contract record, and the TypeScript `Element` type. A target a
caller invokes statically can carry the parameter at no runtime cost, and TypeScript infers it
from the call: the executable `Props` type and factory, and the typegen contract type, are generic
with `= unknown` as the default.

Mechanically: `nx_hir::erase_type_parameters(ty: &ast::TypeRef, params: &[Name]) -> ast::TypeRef`
replaces `TypeRef::Name(p)` for each parameter name with `TypeRef::Name("object")`, preserving
`Array` and `Nullable` wrappers. The `nx-codegen` and typegen models carry the component's
`type_params` beside its props and keep prop types unerased. The IR builder, the C# emitter, and
the TypeScript `Element` emission apply `erase_type_parameters` as they map a prop type; the
TypeScript `Props`, factory, and contract emissions render a parameter reference as its own name
inside a `<TItem = unknown>` parameter list. Both emitters already map `object` to the host top
type, so the erasing paths need no new type mapping.

State and everything derived from it — the executable `<Name>State` type, the `T.Update` record
in the IR and in executable TypeScript, and the typegen `<Name>_update` and `<Name>_state`
contracts — erase in every target, C# and TypeScript alike. A state field may be typed by a
parameter, but the host neither names that instantiation nor receives a value to infer it from: an
NX use site fixed it, and state crosses the boundary as a snapshot. The `nx-codegen` builder erases
an update record's fields against its target component's effective parameters before either
emitter sees them, so the IR and the executable record agree.

An emitted action's payload is the one signature position where a type parameter is rejected
rather than erased. The inline `emits { pick { item:TItem } }` lowers to an action record of its
own, checked and generated outside the component, where the parameter is not a type; contract
resolution in `components.rs`, which knows the effective parameter list including inherited ones,
reports a payload field typed by one with a diagnostic that names the action, the field, and the
parameter. Allowing it later — by checking the payload in the component's type scope and erasing
at the boundary — is a strictly relaxing change.

*Why not erase everywhere:* it is correct, but it throws away the caller-side check in the one
host where keeping it is free. *Why not go generic in C#:* the host does not choose the
instantiation; it deserializes by discriminator, and a generic record cannot be picked from data.
*Why not a generic `Element`:* it is the serializable value, and the wire carries no argument to
fill the parameter with.

Making the `Props` type generic surfaced a latent defect in the executable resolver: an optional
nullable prop is `T | null | undefined` on the input and `T | null` once resolved, and the
resolver returned the input value as it was. It now resolves a key present with no value to
`null`, which is what an absent key already resolved to. No existing test compiled such a prop
under `tsc --strict`.

Generic records, when they come, will map to `Box<T>` in C# as well, because there the host
writes the concrete type at its own deserialization site. That is the line this decision draws:
generics in generated code where the host names the instantiation, erasure where it receives one.

### D7: The type-name resolver for arguments is the checker's, with values excluded by construction

Resolution of `TItem=Contact` calls the same path `resolve_named_type` uses for annotations, so a
component name, a union, an alias, a record, a primitive, and an enclosing parameter all qualify,
and the module's value environment is never consulted. "Reaches nothing" is decided the way an
annotation's undeclared name is decided today, then re-reported with the argument-site wording and
suggestion.

*Alternative considered:* a closed allow-list of records and unions only. Rejected: it would forbid
forwarding an enclosing parameter and primitives, both of which the spec requires.

### D8: No inference in this change

Recorded as a decision so it is not re-litigated during implementation. An explicit argument is
the only way a parameter gets a type; the fallback is `never`. Everything that compiles under this
rule compiles with the same meaning once inference exists, because inference only fills gaps.

## Risks / Trade-offs

- [Adding `'type'` as a property type alternative shifts tree-sitter's conflict resolution and
  breaks a prop named `type`] → Pin `component <X type:string />` and `type X = { type:string }`
  with parser tests before touching the grammar; if the alternative conflicts, scope it to
  `_component_property_definition` only and let records hit the generic parse error.
- [The named "not specified" diagnostic fires for a binding that would have failed anyway] →
  The named diagnostic replaces the mismatch only when the expected type mentions an unspecified
  parameter; a wrong value at an unrelated prop still gets the ordinary mismatch. Covered by a
  test with both kinds of error on one element.
- [Inherited parameter names resolve in the wrong module] → The type scope is keyed by name and
  populated from the effective contract, whose names are unique by rule; the declaring-module
  resolver is bypassed for parameter names only. A test with a base in one module and a derived
  component in another covers it.
- [A type parameter named `string` shadows the primitive and makes it mean two things in one
  file] → Validation rejects a type parameter whose name is in `nx_syntax::PRIMITIVE_TYPE_NAMES`
  or `nx_syntax::BUILTIN_TYPE_NAMES` (`Element`), beside the leading-position rule; the language
  service and typegen read the same two lists. This is deliberately stricter than the rule for a
  module-level `type` declaration, which NX permits to take these names (as `primitive-type-names`
  grants for `never`): a declaration is a site a reader can find, while a type parameter's scope
  silently covers the whole component. Shadowing a declared record, union, or alias stays allowed:
  it follows from D3 and has a scenario.
- [Erasure to `object[]?` gives .NET hosts `List<object>?` where DrawnUI wants `IList`] → Host
  mapping of `object` is a typegen concern that already exists for `object` props; nothing here
  changes it. Noted for the DrawnUI binding, not solved here.
- [`never?` at a nullable parameter site accepts only `null`] → Intended; the spec has a scenario
  so it reads as designed rather than as a bug.
- [Language service surfaces a `Type::Parameter` in hover or completion text] → It renders as its
  name, which is the right hover text. A smoke test over a generic component in the language
  service suite guards against panics.
- [A parameter's invisibility outside its component is hard to observe] → NX does not report an
  undeclared name in a type annotation today; it becomes an origin-less nominal type that no value
  satisfies. The spec's scenario therefore shows the top-level name resolving to what the module
  declares under it, rather than an "undeclared type" diagnostic this change does not add.
- [A component body is one expression, so "annotation inside the body" has one site] → the
  scenarios use a `state` field's annotation and default, which the body scope checks with the
  same type-parameter scope as the rendered expression.

## Migration Plan

Additive, with one deliberate change to existing generated output. No existing program declares a
prop typed `type`, and the IR version does not move because the IR shape is unchanged for programs
without type parameters. The executable resolver fix in D6 does change output for every component
with an optional nullable prop that has no default: a key present with `undefined` now resolves to
`null` rather than being passed through, so `JSON.stringify` of the element keeps the key as
`null` where it previously dropped it. A codegen test pins that on a non-generic component. No
rollback steps beyond reverting the change.
