## Context

Type references in declarations are lowered as `ast::TypeRef::Name` and resolved by the checker
(`type_from_type_ref_in` and its quiet variant in `crates/nx-types/src/infer.rs`). A name that resolves
to nothing becomes a `Type::Named` with no declaration behind it, which satisfies only itself, so the
problem shows up later as a mismatch or not at all. The one position that already checks is a type
argument at an element (`<Range T=Contatc/>`), which uses `is_visible_type_name`, `visible_type_names`
and `closest_candidate` to report `unresolved-type-argument` with a suggestion.

Unresolved `Type::Named` is also a legitimate intermediate state today: the comment on
`nominal_type_in_module` notes that resolving a library declaration's type in the consumer's scope is
what used to leave it unresolved. Any new check must therefore resolve in the declaring module's
namespace and must not report references it merely failed to re-resolve from elsewhere.

## Goals / Non-Goals

**Goals:**
- One diagnostic, `unresolved-type`, for every written type reference that names no visible type, in
  every declaration and annotation position.
- Report each reference once, in the module that wrote it, with a suggestion.
- Library loading fails for a library whose own declarations name an unresolved type.

**Non-Goals:**
- Changing how resolved types behave, or the `unresolved-type-argument` diagnostic.
- Inferring or auto-importing a type a module forgot to import (the suggestion names it; the author
  adds the import).

## Decisions

### D1. Check at the point a written reference is resolved, in the declaring module
The check runs where the checker turns a written `TypeRef::Name` into a `Type` for a declaration or
annotation of the module being analyzed, using the same visibility test the type-argument check uses.
References resolved on behalf of another module (an imported declaration's fields, a contract walked
across modules) go through the quiet path and are never reported by the consumer: the declaring
module reported them when it was analyzed, and a library with such an error does not load.

*Alternative*: validate every `TypeRef` in a separate pass over lowered declarations. Rejected: it
would duplicate the scope rules (type parameters, derived `Property`/`Update` names, built-ins, the
prelude) that the resolution path already encodes.

### D2. What counts as visible
Primitive names; declarations and imports visible in the module (records, unions, aliases,
components, actions); built-in types (`Range`) and prelude declarations; type parameters in scope
(record and component `T:type` parameters; a function type declares none, and validation rejects a
`T:type` written in one); and derived companions
(`Name.Property`, `Name.Update`) of a visible declaration. A union case is a value of its union, not
a type: `Shape.circle` in type position is reported, with a message naming `Shape` as the type to
write, rather than a spelling suggestion.

### D3. Diagnostic shape
`unresolved-type`, error severity, primary label on the reference, message
"`<name>` is not a visible type", with "; did you mean `<candidate>`?" when a visible name is close
enough to be the one meant. Reported once per reference.

A `TypeRef` carries no span, so lowering records each type name it lowers with where it was written,
and each function return type and value annotation with its own span. The report narrows the
enclosing construct's span (the field, parameter or annotation) to the name inside it, falling back
to the construct for a module built without spans. `unresolved-type-argument` is labelled the same
way. A reference that writes one name more than once (`<function a:Dup />: Dup`,
`<Pair A=Dup B=Dup/>`) is told apart by occurrence: the walk visits a reference's names in the order
they are written and counts each name as it goes, so the report for the second visit underlines the
second occurrence. Each entry into a reference of its own (an annotation, an alias's target) counts
afresh.

The suggestion is shared with `unresolved-type-argument` and stricter than the general
`closest_candidate`, because type names can be short: letter case is not an edit, the edits allowed
are a third of the shorter name (at least one), and they must leave some of the written name in place
— `Strin` suggests `string` and `Txt` suggests `Text`, but `Txt` is not "corrected" to `int`, nor `T`
to `A`.

### D4. Positions that already report keep their diagnostic
An `extends` clause naming nothing is already a `lowering-error` ("Record 'User' extends 'Bse', but
'Bse' could not be resolved"), and a type argument is already `unresolved-type-argument`. Both stay as
they are and are not reported a second time as `unresolved-type`. Likewise a payload field of an
inline emit typed by the component's type parameter is already rejected by component resolution, so
in that component's inline payloads its type parameters resolve to an error without a report; every
other name those payloads write is still checked. An imported or workspace-peer alias's target is the
declaring module's to report, so a consumer walking it reports no `unresolved-type`. (The walk still
resolves the target's names in the consumer's namespace, a pre-existing behavior this change leaves
as it was: an applied target the consumer cannot resolve still yields that applied type's own error.)

### D5. What is not a written reference
Two things reach the checker as type names without being names the author wrote, and neither is
reported:

- **Error recovery.** Lowering stands in for a type it could not lower — a syntax error, a missing
  type, a construct post-parse validation already rejected — with `TypeRef::recovery()`, whose name
  is empty and cannot be written. The checker resolves it to an error silently; the syntax or
  validation error is the one report. A use, inside the same function type, of a `T:type`
  parameter validation rejected there lowers to the stand-in too.
- **A failed import's names.** A name an import would have made visible, had it resolved, is the
  import's error, not a missing type: a selective import's names (and their `.Property`/`.Update`),
  a namespace import's `Alias.*` names, and — since its names cannot be known — any name, for an
  unaliased wildcard import that did not resolve. Preparation records each import that did not
  resolve (missing, ambiguous, unparsable target, unsupported or invalid path, or local resolution
  skipped because the importing file's own path could not be resolved), so an import that resolved
  but carries another diagnostic, such as being written twice, hides nothing. An import whose target
  module lost a top-level declaration to an error is recorded the same way: the target keeps only the
  declarations that survived, and its error is the report, so a shared module mid-edit does not
  light up every module that imports it. In a module with an error, a declaration is lost when an
  error lies outside every declaration that lowered (an unclosed `export type Contact = {` at the
  end, a removed `enum` form), or when a line begins a declaration (optional `export`/`private`,
  `abstract`, `external`, then `type`, `action`, `let`, `component` or `enum`, from the first
  column) where no lowered declaration begins — an unclosed declaration before it ran on over it.
  The second test scans the text, since a swallowed declaration is no longer a node of its own. An
  error inside a declaration that lowered and swallowed nothing — a validation error such as
  `tags:string[]`, a syntax error in a field type or a function body — leaves every name bound, so
  it hides nothing. What such an
  import did bind stays bound. The same applies
  to the tag of an applied type and to a type argument.

### D6. Sweep the repository
Run the full Rust, .NET, TypeScript and example suites; fix any example, fixture or test source that
named an undeclared type. Validate ReachMe's built-in libraries, templates and seeds through the wasm
SDK before release.

## Risks / Trade-offs

- [False positives from a scope the check does not know about] → Reuse the resolution path's own
  scope rules (D1) and add a scenario per kind of visible name (D2).
- [Cascades onto an existing error] → Recovery stand-ins and failed imports' names are not written
  references (D5). A site whose type is already an error also takes a bare name silently, rather
  than saying it "expects <error>".
- [An unaliased wildcard import that fails hides every unresolved type in its module] → Accepted:
  the module already has an error, and the hidden names reappear, if still wrong, once it is fixed.
  The same holds while the imported module has lost a declaration to an error: the importer's own
  typos reappear once that declaration parses again. An error that leaves every declaration in
  place hides nothing. The rule errs toward hiding: stray tokens between declarations are an error
  outside every declaration, and a first-column line inside a broken module that merely looks like
  a declaration (say, in a multi-line text block) counts as a swallowed one; either hides the
  importer's unresolved names until the target is fixed, never reports one that should not be.
- [Double reporting across modules] → Consumers use the quiet path for other modules' references.
- [Breaking existing sources] → Intended; the spec change is marked BREAKING and the sweep (D6)
  finds in-repository cases.

## Migration Plan

Ship in the next NX release with a release note; hosts pinning NX see the new errors only when they
bump the pin.

## Open Questions

- None.
