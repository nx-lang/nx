## Context

NX currently has two conflicting stories for declaration body content.

- Older docs and examples describe a dedicated `content` slot concept for record-like declarations.
- The current implementation instead uses `children`-based special cases for some declaration and
  runtime paths rather than preserving the language-level `content` concept directly.
- Ordinary text runs in element bodies are still dropped during HIR lowering, which means examples
  like `<Foo>label text</Foo>` are not representable today even though they are central to the
  requested feature.

The requested language surface is explicit: a declaration author can mark at most one property as
the content property with the contextual keyword `content`, callers may supply that property either
through element body content or as a normal named property, and `content` must remain a usable
identifier everywhere else. The user also wants the marker available wherever NX already has
function-like or record-like property syntax, rather than limiting it to only a subset of those
surfaces.

This is a cross-cutting change. The parser must preserve the marker, lowering and exported
interface metadata must carry it, record inheritance must not accidentally synthesize multiple
content properties, type checking must bind body content to the declared property consistently, and
runtime evaluation must construct record/function/component arguments the same way. Docs, grammar
references, and tests also need to move in the same change because the existing examples still
reflect the older slot model or the implicit `children` behavior. There is no need for backward
compatibility, so the design can remove older `children`-based implementation paths directly rather
than layering the new behavior on top of them.

## Goals / Non-Goals

**Goals:**
- Allow every function-like or record-like declaration surface to mark exactly one property with
  contextual `content`: plain records, action records, paren-style `let` parameters, element-style
  `let` parameters, component props, inline emitted-action payload fields, and component state
  fields.
- Preserve the content-property marker through CST, HIR, record-shape resolution, and exported
  library interfaces.
- Bind markup body content to the declared content property for NX-defined record construction and
  function/component calls, including paren-style functions invoked with markup syntax.
- Keep the content property callable through normal named-property syntax as well.
- Reject calls that provide the same content property both by body content and by explicit named
  property.
- Treat `content` as contextual only in content-property declaration position so it remains a
  normal identifier elsewhere.
- Preserve plain text body content so scalar content properties such as `label:string` can receive
  `<Foo>label text</Foo>`.
- Replace `children`-based implementation terminology for element body handling with `content`
  terminology throughout the affected compiler and runtime layers.

**Non-Goals:**
- Preserve the old implicit `children` binding behavior or old terminology in parallel with the new
  feature.
- Add multiple named slots.
- Add backward-compatibility parsing for older standalone slot syntax forms.

## Decisions

### 1. Add a content-aware property-definition form everywhere NX already has function-like or record-like properties

The grammar will gain a content-capable property-definition form that accepts:

```nx
content label:string
```

and preserves the marker on the resulting `PROPERTY_DEFINITION` node. That content-capable form
will be used in every declaration surface that already models typed properties:

- plain record declarations (`type Name = { ... }`)
- action declarations (`action Name = { ... }`)
- element-style `let <Name ... />` signatures
- paren-style `let name(...)` parameter lists
- component signatures
- inline emitted-action payload fields
- component state groups

This keeps `content` contextual rather than reserved. `content:string` continues to mean a property
or parameter literally named `content`, while `content content:string` remains available when an
author intentionally wants the content property itself to be named `content`.

Alternative considered:
- Restrict `content` to only plain records and element-style `let` signatures.

Why rejected:
- Components are specialized functions and emitted-action/state fields are specialized records. Paren
  functions can already participate in markup-style invocation. Artificially forbidding the marker
  in those places would create unnecessary inconsistency for equivalent declaration shapes.

### 2. Carry the content marker as field/parameter metadata, not as another name-based convention

HIR and interface metadata will record the designation directly on the affected property/parameter
entry rather than inferring content semantics from a name-based convention.

The intended model is:

- `nx_hir::Param` gains `is_content: bool`
- `nx_hir::RecordField` gains `is_content: bool`
- exported/imported interface metadata in `nx-api` mirrors that bit for functions, components, and
  records
- helper APIs expose the single effective content property for a declaration or resolved record
  shape

Field-level metadata makes record-shape resolution, copying, import/export synthesis, and later
diagnostics straightforward because the designation travels with the property it belongs to.

Alternative considered:
- Store a declaration-level `content_property_name: Option<Name>` separately from fields.

Why rejected:
- The name would need to be re-looked-up and revalidated every time shapes are copied, inherited,
  or converted through library interfaces. Keeping the bit on the field/param is simpler and less
  error-prone.

### 3. Record-like surfaces preserve at most one effective content property

Plain records already support inheritance from abstract bases, and component-emitted action payloads
plus component state groups behave like record-shaped field lists. Content-property semantics must
therefore apply consistently wherever NX treats a declaration as record-like, and record
inheritance must preserve the rule at the effective-shape level rather than only within one source
block.

The resolved record shape rules will be:

- if an abstract base record marks one content property, derived records inherit that designation
- a derived record that does not introduce a new content marker continues using the inherited one
- a derived record that marks another content property is invalid because the effective shape would
  have more than one content property

This keeps the "at most one" rule true for actual instantiable record shapes rather than only for
single source blocks.

Alternative considered:
- Let derived records override the inherited content property.

Why rejected:
- Record inheritance already rejects duplicate field redeclarations, so there is no clean override
  mechanism today. Allowing a second marker would make content-property resolution order-dependent
  and harder to reason about.

### 4. Use explicit content-property binding for all NX-defined markup-style calls

Type checking and runtime evaluation for NX-defined markup-style invocations will stop using
`children`-based special cases and will instead ask "does this declaration expose a content
property, and what is its name/type?"

Concretely:

- `nx-types` element binding analysis will build an `ElementBindingSpec` around
  `content_property: Option<(Name, Type)>` plus the full named-property map
- element body content will bind to the declared content property's type
- the named property form remains legal
- providing both sources at once becomes a dedicated content-binding conflict
- if body content is present but the target declaration has no content property, the invocation is
  rejected
- the same rule applies whether the callee was declared with element-style `let`, paren-style
  `let`, `component`, or record construction syntax

This removes hidden body-binding conventions and makes declaration authors opt in explicitly.

Alternative considered:
- Preserve old `children`-based fallback behavior when no explicit content property is present.

Why rejected:
- That would keep two competing body-binding rules in the language at once and undermine the point
  of the new explicit marker.

### 5. Lower ordinary element text runs into string literal content expressions

Supporting `<Foo>label text</Foo>` requires more than renaming a binding hook. HIR lowering
currently drops plain text runs from ordinary element bodies, which means no later phase can bind
them to a `string` content property.

The change will therefore make `lower_element_children` preserve ordinary text runs as string
literal expressions, reusing the existing literal/string evaluation path. That produces consistent
content sequences for:

- scalar text content
- nested element bodies
- mixed text and element content
- existing intrinsic/native element body content

This is intentionally broader than the narrow content-property hook, but it is the smallest
cross-phase change that makes the requested syntax real without target-sensitive lowering.

Alternative considered:
- Only lower plain text specially when the callee later resolves to an NX-defined declaration with
  a `string` content property.

Why rejected:
- Lowering happens before full binding, imported declarations complicate target discovery, and
  target-sensitive body lowering would make the same markup body mean different things depending on
  later resolution.

### 6. Rename the generic body-content channel to `content` across the implementation

The explicit content-property feature applies to every NX-defined function-like or record-like call
site, but intrinsic/native elements still need a generic body-content channel. The implementation
should use `content` for that concept consistently rather than keeping `children` as the internal
name for the same source-language idea.

Runtime behavior will therefore split into two paths:

- resolved NX-defined record/function invocations inject normalized body content under the declared
  content-property name
- intrinsic/native element values preserve normalized body content under a generic `content`
  channel when no declaration-owned content property exists

This aligns the implementation vocabulary with the language surface and removes the last major
`children`-named special case from body-content handling.

Alternative considered:
- Keep `children` as the implementation name for intrinsic/native element body content while only
  changing NX-defined declaration binding.

Why rejected:
- The user request is to make `content` the consistent term for element body semantics throughout
  the implementation. Keeping `children` internally for the same concept would preserve avoidable
  conceptual drift.

### 7. Update diagnostics, docs, and tests in the same change

The current repository contains legacy docs that still describe standalone `content` slots and code
paths/tests that still speak in terms of `children`. The implementation change should update:

- parser fixtures and CST/lowering tests for content-marked properties
- type-checker and interpreter tests for body-content binding, text content, named-property
  fallback, and double-supply errors across records, functions, and components
- grammar/reference docs and tutorials to show `content` as an inline property modifier

Alternative considered:
- Land parser/runtime changes first and update documentation later.

Why rejected:
- This feature is source-language-facing, so leaving the docs on the old model would make the
  language definition inconsistent immediately.

## Risks / Trade-offs

- Lowering plain text runs into string expressions changes the observable child sequence of
  ordinary elements. -> Mitigation: add parser, type, and interpreter tests that pin the new mixed
  text/element behavior explicitly.
- Record inheritance can now fail for a second kind of effective-shape conflict. -> Mitigation:
  validate duplicate content-property designation alongside existing inherited-field checks.
- `content content:string` is legal but visually unusual. -> Mitigation: document the contextual
  keyword rule and prefer examples with non-`content` property names unless the distinction matters.
- Imported library interfaces must preserve the new metadata or local/imported behavior will drift.
  -> Mitigation: add interface round-trip coverage in `nx-api` tests for function params,
  component props, and record-like fields marked as content.

## Migration Plan

1. Update grammar, generated parser artifacts, syntax metadata, and highlighting so every
   content-capable declaration surface can mark `content` properties without reserving `content`
   globally.
2. Extend HIR, record-shape resolution, component metadata, and library interface artifacts to
   preserve the content marker and enforce at most one effective content property where applicable.
3. Replace `children`-specific binding/evaluation logic and terminology in `nx-types`,
   `nx-interpreter`, and related runtime artifacts with explicit content-property semantics.
4. Preserve ordinary text body runs during lowering so text body content participates in the new
   binding path.
5. Update diagnostics, docs, parser fixtures, and runtime/type tests to reflect the explicit
   content-property model across records, functions, components, and intrinsic/native elements.

## Open Questions

No blocking questions.
