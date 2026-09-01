## Context

See `proposal.md` — Why. The load-bearing facts about the current implementation:

- `Type::Named(Name)` is the *unresolved* fallback. `resolve_named_type` returns `Type::Union(..)`
  when the name is visible and falls through to `Type::Named` when it is not, so a single variant
  means both "a nominal type I know" and "a name I could not resolve". `UnionType` carries only a
  name, its case names, and an optional base.
- `Type` derives `PartialEq`/`Eq`/`Hash`, and `is_compatible_with` opens with `self == other`.
  `Type::UnionCase` vs `Type::Union` compares `case.union == union.name`. Type identity is therefore
  structural over names today.
- `Type` is not serialized. It is converted to `ast::TypeRef` for publication
  (`type_to_type_ref`), which collapses every nominal type to `TypeRef::name(..)`. `TypeRef` is the
  *syntax* node, also used for parsed source, with ~105 construction sites. A module's contract is
  therefore published as syntax and re-resolved by each consumer.
- A canonical address already exists and is already used: `(RuntimeModuleId, LocalDefinitionId)` with
  `LoweredModule::item_by_definition`. `resolve_item` *ends* at that address; names are only how it
  finds it. `CodegenExpressionKind::UnionCase` already takes a canonical `CodegenReference`.
- `contextual-literal-binding` resolves a declaration's property type references in the declaring
  module (`type_from_type_ref_in` / `nominal_type_in_module`), so expected types at imported
  components are already correct. Its rewrite target is `Member { base: Ident(type_name), member }`,
  resolved downstream by visible name — the part that does not survive.

Two constraints worth stating up front. First, there is no substitute *spelling*: under a wildcard
alias `ui.Fit.cover` does not lower either, so this cannot be solved by emitting a different name.
Second, constant cases are not literals in generated code — for a constant union JS emission
produces `Fit.cover` plus an emitted import — so they cannot be constant-folded away.

## Goals / Non-Goals

**Goals:**

- Nominal identity is origin-based, and that origin survives from analysis through lowering, code
  generation, evaluation, and published interfaces.
- A resolved contextual name lowers without the declaring type being nameable at the use site.

**Non-Goals:**

- Changing NX's *source-level* import rules. A name an author writes still resolves lexically; origin
  is how the system reaches a definition it has already resolved, not a new way for authors to skip
  imports when writing a qualified name.
- Structural typing. Nominal types stay nominal; this narrows what counts as "the same one".
- Generic or parameterized nominal types.
- Making `ui.Fit.cover` lower. Multi-segment member access is a separate gap (NXE14); this change
  removes the need for it at typed sites rather than implementing it.

## Decisions

### D1: Origin is `(module identity, definition id)`, carried on the nominal type

`UnionType`, `UnionCaseType`, and `NamedType` — the last covering records and components — gain a
declaring origin; `Name` stays for display. The pair is the address the runtime already resolves
to, so nothing new needs inventing and the mapping to `CodegenReference` is direct.

`DeclaringOrigin` lives in `nx-hir` rather than `nx-types`, because the inheritance chains that
decide record and component identity are resolved there. The peer namespace entry
`contextual-literal-binding` introduced was the same pair under another name and is now the same
type.

*Alternative — intern nominal types into a global table and compare by handle.* Cheaper equality and
no field growth, but it needs a table threaded through every context that builds a type, including
ones that build types before a program exists (single-file analysis, the language service). Rejected
as more invasive for the same result.

*Alternative — keep names and add a module prefix (`widgets.nx::Fit`).* Simple, but it makes identity
depend on how a path was spelled and reintroduces string comparison as the identity test.

### D2: `Type::Named` carries origin rather than becoming an unresolved marker

**Revised while implementing group 5.** The original D2 said `Type::Named` should mean only "a name
resolution reached nothing for". That is available for unions, which have their own variant, but not
for records and components: they *are* `Type::Named`, alongside the built-in `Element` and `object`
names. Narrowing the variant to unresolved names would have meant introducing two more variants and
splitting every match site three ways.

So `Type::Named` holds a `NamedType { name, origin }` instead. A record or component name reaches a
declaration and carries it; `Element`, `object`, and a name that reaches nothing stay origin-less.
Equality is the same rule the union types use — two origins are the same declaration or they are
not, a name decides only where neither side has an origin — so the variant no longer matches on
spelling wherever a declaration was reachable, which is what the proposal's soundness bug needed.

Equality changes as a consequence: `self == other` on two nominal types now compares origin. This is
the highest-risk part of the change and is why it is not bundled with the syntactic work of
`contextual-literal-binding` (see that change's `review.md`).

### D3 (revised): A contract is resolved in the namespace of the module that wrote it

**Revised while implementing group 3.** The original D3 said publication must carry a resolved type
representation because a loaded library, unlike a workspace peer, does not expose its
`LoweredModule`. That is not what the code does: `add_imported_interface_bindings` registers *every*
module of an imported library as a peer of the consuming module, so a library's own definitions are
already readable the same way a workspace peer's are. A probe confirmed it — a consumer importing
only `Img` from a built library already reports against the library's own cases, with no interface
change at all.

The gap the original D3 was aiming at is real but one level further out. A declaration's type
references are written in the namespace of the module that wrote them, and that namespace is not
just that module's own items — it includes what that module imported. `nominal_type_in_module`
searched only the declaring module's top-level items, so a property typed by a union the declaring
module *imported* resolved to nothing there and fell back to the consumer's namespace. That fallback
is the same soundness hole one module further along the chain, and it is silent:

```nx
// widgets.nx: export type Fit = fill | contain | cover
// panel.nx:   import { Fit } from "./widgets.nx"
//             export let <Panel fit: Fit = {Fit.fill} /> = <div fit={fit} />
// app.nx:     import { Panel } from "./panel.nx"
//             type Fit = stretch | squish
//             let root() = { <Panel fit=stretch /> }   // accepted, no diagnostic
```

So the fix is to resolve the reference where it was written, not to publish the answer. A module's
prepared bindings *are* its namespace, so each module's type namespace is collected once and handed
to every module that has it as a peer (`PreparedModule::add_peer_type_namespace`,
`TypeNamespaceEntry`). Both the workspace graph and the library build now prepare every module
before analyzing any of them, because a peer's namespace is only known once its own imports have
been applied.

*Alternative — publish a resolved type representation in the interface as well.* Rejected as a
second mechanism for an answer the first already has. The peer path is tried first, so the published
form would never be the one consulted: it would ship untested and have to be kept in sync by hand.
It becomes the right answer only if a library artifact ever stops carrying its lowered modules,
which is the point at which to build it.

*Alternative — stop falling back to the consumer's namespace and leave the reference unresolved.*
Sound, and a two-line change, but it trades a silent unsoundness for a loud incompleteness: a
legitimate `<Panel fit=fill />` would be rejected along with the collision.

**Amended while fixing review finding RF3.** The rule reaches type *aliases* too, and it had not.
`foreign_nominal_type` followed a union, a record, and a component and stopped at an alias, so a
property typed by a name the declaring module had aliased fell through to the fallback — the
consumer's namespace — with no diagnostic anywhere:

```nx
// widgets.nx: export type Fit = fill | contain | cover
//             type FitAlias = Fit
//             export let <Img fit: FitAlias = {Fit.fill} /> = <div class="img" />
// app.nx:     import { Img } from "./widgets.nx"
//             type FitAlias = stretch | squish
//             let root() = { <Img fit=stretch /> }   // accepted, `stretch` reached the IR
```

An alias is a type reference like any other, so what it names is resolved in the module that wrote
it, recursively, guarded by a stack of the alias declarations currently being followed so a cycle
among them terminates. The declaring module reports its own alias cycle; the guard only stops the
consumer from following one forever. Following the alias also fixes the *legitimate* case, which was
broken in the other direction: with no same-named alias in the consumer, the reference resolved to
nothing and `<Img fit={Fit.cover} />` was rejected outright.

### D4: The contextual rewrite targets a reference node, not a name

`apply_contextual_name_resolutions` currently rewrites to `Member { base: Ident(type_name), .. }`.
It should instead produce a node carrying the resolved origin, so codegen can build a
`CodegenReference` directly and the interpreter can reach the definition without `resolve_item`.

This refines decision D2 of `contextual-literal-binding` (resolve after analysis, rewrite once)
rather than reversing it: the post-analysis rewrite stays, only its target changes. The
bare-vs-qualified IR equivalence test from that change is the guard — the qualified form must
continue to produce byte-identical IR.

*Alternative — lower constant cases to literals.* Attractive (an unbraced value is a literal, per
that change's core invariant) and enough for the interpreter, which only needs the union and case
names. Rejected because JS emission of a constant union produces a real cross-module reference with
an import; folding to a literal would change generated code and break the equivalence guarantee. It
also does not help a case with a payload, or one whose union declares a base, since those need the
definition for field defaults.

### D5: Diagnostics name the declaring module only when it disambiguates

`expects Fit, found Fit` is the current output for two same-named types. Qualifying every nominal
type in every message would be noise; qualify when two types in one message share a display name.

**Amended while fixing review finding RF4.** "Share a display name" is decided on the nominal parts,
not on the rendered strings. The first implementation qualified only when the two sides rendered
*identically*, which left the qualified form uncovered: a `Type::Union` renders as `Fit` and a
`Type::UnionCase` as `Fit.cover`, so `expects Fit, found Fit.cover` — two different strings, and
exactly as ambiguous as the identical pair — was never qualified. Each side now contributes the
declarations it names (a union case contributing its *union's* name, which is the name the reader
has to tell apart), and the pair is qualified when one name covers two declarations.

The same reasoning applies to a message that names only one type. `'stretch' is not a case of union
'Fit'` names the expectation's `Fit` while the author is looking at their own, which does declare
`stretch`; the union's name is qualified there when a different union of that name is visible at the
use site, and left bare when there is nothing to tell it apart from.

### D6: A host-supplied type name is a label, read in the module that declared the expectation

**Added while implementing group 10.** Inside a program, analysis rejects a same-named substitution
before evaluation. At a **host input boundary** it cannot: the value comes from the embedder as a
`type_name` string and a field map, and an embedder structurally *cannot* supply an origin — it has
no way to name one. So the actual side of that comparison will never carry identity.

The fix is therefore not origin on `Value::Record`. Putting it there would mean the ~218 record-value
construction sites RF2 counted, for no gain at the only boundary that matters. The fix is on the
**expected** side, and it is the same rule as D3 one layer down: *a type reference is resolved in the
module that wrote it, at runtime as well as during analysis.*

That settles what a host's `type_name` means. It is a **label**, resolved in the namespace of the
module that declared the expectation — the property's or emitted action's own module — and never in
the module that bound the handler or constructed the element. It selects among the declarations that
module can see; the declaration it reaches then governs construction, and the lineage is compared by
declaration, exactly as the static half now does. A consumer's same-named type is simply not a
candidate, because it is not in the namespace being read.

Two things were missing for that rule to hold at runtime. `Value::ActionHandler` recorded the module
that *evaluated* the binding, so `validate_handler_input` resolved the expected action there; it now
carries the emit's declaring module, threaded from `EffectiveEmit`. And `runtime_prepared_module`
registered peer *modules* without their namespaces, so a foreign declaration's `extends` clause could
not reach a base that peer had imported — the same gap D3 closes for analysis, unfixed at runtime
because nothing had yet asked the question there.

The same rule reached one step further than expected. `Interpreter::initialize_component` and
`evaluate_component` took the module the *caller* named the component in, but `Component::body` and
its prop defaults are `ExprId`s into the *declaring* module's arena — so evaluating an imported
component read a different expression at the same index. That is pre-existing and has nothing to do
with nominal identity, but it made this change's own rule untestable for the ordinary embedding
shape, a handler bound on a component the app imported rather than extended. It survived because the
program-entrypoint API (`initialize_resolved_component`) resolves the declaring module through
`entry_component`, so every existing library test went around the module-taking API. Fixed here
rather than deferred: it is two lines, it is the same sentence as the rest of the group, and leaving
it would have shipped a boundary fix whose main use is unreachable.

*Alternative — keep comparing names at the boundary and rely on analysis.* This is what the code did.
It is sound for values a program constructs and unsound for values it receives, which is precisely
the case the boundary exists to handle.

*Alternative — reject any host value whose label is not the exact expected name.* Sound and trivial,
but it removes subtyping at the boundary: a host could no longer supply a descendant of an abstract
base, which is how component props typed by an abstract record are meant to be used.

### D7: A resolved type re-enters lookup by origin, not by its spelling

**Added while fixing review finding RF1.** Carrying origin on `Type::Union` and `Type::UnionCase`
settles *equality*, but several paths that run after a type has resolved went back to a
name-keyed map to ask a second question about it — what the union's base is, what a member's type
is, what two cases have in common. `union_defs` is keyed by the name the module reaches a union
*by*, so a consumer declaring its own `Shape` gets its own entry back for a foreign `Shape`, and
the origin the type had been carrying is discarded at exactly the point it was needed:

```nx
// widgets.nx: export abstract type Drawable = { ink:string = "black" }
//             export type Shape extends Drawable = circle | square
//             export let widgetShape: Shape = {Shape.circle}
//             export let <Ink d: Drawable /> = <div ink={d.ink} />
// app.nx:     import { Ink, widgetShape } from "./widgets.nx"
//             type Shape = circle | square         // no base
//             let root() = { <Ink d={widgetShape} /> }   // rejected: the local `Shape` answered
```

The rule is that a type which already names a declaration never re-derives it from a spelling. One
private selector (`union_entry_for`) takes the name and the origin, and every post-resolution
lookup goes through it: base-through-abstract-record compatibility, shared and case-specific member
access, and common-supertype construction. The name is still tried first as a fast path, because it
reaches the same entry whenever there is no collision; the scan over both maps is what covers one.

Two further leaks of D3's rule surfaced while building the test vehicle for this, and are fixed with
it. A union's *base* and its cases' *field types* are names the union's own module wrote, so they
resolve there — previously they resolved in the consumer, which is the same capture one level down.
And an imported value's type annotation was resolved in the consumer's namespace rather than the
declaring module's, so `export let widgetShape: Shape` acquired whatever `Shape` meant at the use
site. That last one is why the collision was hard to reach from a test at all: the value handing the
foreign union across the boundary was itself being re-typed on arrival.

*Alternative — key `union_defs` by origin instead of by name.* Rejected: the map's job is to answer
"what does this spelling mean here", which is what source-written names need. The two questions are
different, and the fix is to ask the second one explicitly rather than to make the first one worse.

**Amended while fixing RF1's reopening.** Asking the second question explicitly is not enough while
the *foreign* cache is still keyed by a spelling. `foreign_union_defs` holds the unions reached
through another module's signature, and nothing in a consumer names them, so its key was never a
namespace answer — it was the name the type happened to arrive under. Two peers each exporting a
`Shape` are two declarations arriving under one key, and the second erases the first; both values
keep their distinct origins, so the selector then finds no entry at all for the erased one and the
value is rejected. That map is therefore keyed by `DeclaringOrigin` — the question it actually
answers — and the selector addresses a foreign entry by origin outright. Only `union_defs` keeps a
name key, because a name key is what it is for.

The same rule reaches one more place D3 had not covered: an imported *function's* parameter and
return annotations. They are names the declaring module wrote, exactly like a value's annotation,
and resolving them in the consumer let an unrelated local declaration become an imported function's
signature. Both the workspace-peer path and the library-interface path now resolve them where they
were written.

### D8: An inheritance walk is keyed by declaration, like everything else

**Added while fixing review finding RF5.** The record and component inheritance walks — the cycle
stacks, the memoized validation statuses — were keyed by `Name`. That was correct while a lineage
lived in one module. This change makes a cross-module lineage a first-class case, and a name-keyed
stack reads the second link of one as a repeat visit:

```nx
// base.nx: export abstract type Shape = { ink: string }
// app.nx:  import { Shape as ui.Shape } from "./base.nx"
//          type Shape extends ui.Shape = { r: int }
//          error: Record inheritance cycle detected: Shape -> Shape
//          error: Record 'Shape' has no field 'ink'
```

Both walks are keyed by `DeclarationKey` — the declaration, or the spelling where a context reached
none, which mirrors `same_declaration`'s `(None, None)` arm. The name is still carried alongside,
because the cycle a diagnostic prints is a chain of names. `validate_record_definition` and its
component twin also now take a resolved definition rather than a bare one, which fixes a second
name-keyed hole in the same function: they were resolving an imported base's *own* base in the
consumer's namespace.

### D9: One rule for "same declaration", written once

**Added while fixing review finding RF8.** The origin comparison existed in three copies — one in
`nx-types::ty`, one named `same_record_declaration` in `nx-hir::records`, and one inline in
`union_entry_for` — each encoding `(Some, Some) => eq / (None, None) => name / _ => false`. A rule
written once per kind drifts, and a kind whose copy drifts is a kind where a same-named foreign
declaration is accepted again. It is now one `nx_hir::same_declaration`, beside `DeclaringOrigin`,
called by records, components, named types, unions, and union cases alike — which also removes the
oddity of `is_component_subtype` calling a record-named helper.

*Not done: folding the record and component machinery into one generic.* The pairs really are
parallel, but they differ in their `Item` variant, their definition struct, and their error enum,
and unifying them would trade duplication that reads plainly for machinery that does not — in a
change whose subject is elsewhere. Worth its own change if a third kind ever appears.

## Risks / Trade-offs

- **Equality semantics change program-wide, and failures are silent rather than loud.** → Land D1/D2
  first, on their own, with the existing suite green before anything else moves. The same-name
  collision case is the canary and should become a test before the fix.
- **Every module must be prepared before any is analyzed, so a peer's namespace exists when it is
  needed.** → Preparation is not the expensive phase; type checking is, and it still runs once per
  module. The graph and the library build were each already preparing every module exactly once, so
  the restructuring reorders that work rather than repeating it.
- **Origin makes types unequal that used to be equal, so previously accepted programs may now be
  rejected.** → That is the point where the rejection is the collision bug, but it can also surface
  where a type is legitimately reached twice by different routes. The "one type reached under two
  names is one type" scenario in the spec exists to pin that down.
- **The language-service change (RF3) depends on an import graph the service does not build today.**
  → Sequence it last; it consumes the same resolution the compiler uses rather than adding a parallel
  mechanism.
- **Record and component identity is a subtype relation, not equality, so a wrong answer can
  over-reject a legitimate lineage as easily as it can accept a bogus one.** → Every rejection
  scenario is paired with a positive one asserting that the declaration a contract actually names
  still satisfies it, including a base the consuming module cannot spell.
- **Reading a host label in the declaring module narrows what a host can name.** → A label that
  module cannot see now resolves to nothing rather than to whatever the evaluating module happened
  to call it. That is the intent, but it makes previously accepted host input fail, so each
  boundary rejection scenario is paired with a positive one — including a descendant declared in a
  third module, which is the case that first surfaced the missing runtime peer namespaces.

## Migration Plan

Sequenced so each step is independently verifiable:

1. Origin on unions and union cases, and origin-based equality (D1, D2), workspace peers only. The
   collision test flips from accept to reject.
2. A declaration's nominal types resolve in the namespace of the module that declares it, including
   what that module imported (D3 revised).
3. Contextual references carry origin through lowering and codegen (D4). The import guidance
   `contextual-literal-binding` emits is removed, and its deferred spec scenarios begin passing.
4. Language-service lookup through the import graph, and diagnostic disambiguation (D5).
5. Origin on records and components, and origin-based inheritance chains (Q2). Steps 1–4 built the
   machinery this reuses, which is why it is last rather than beside step 1.
6. The host input boundary (D6): the expected declaration resolves where it was declared, at runtime
   too. Last because it reuses step 5's subtyping wholesale and only has a question to answer once
   the static side is settled.

Rollback is per-step; steps 3–4 each depend on the ones before but not on each other.

## Settled Questions

### Q1: `Type::Named` is retained, and carries origin — settled in tasks 2.4 and 5.1

Removing it outright is not available while records and components are still reached by name:
`Type::Named` is the representation for both, and for the built-in `Element` and `object` names. So
it stays. It was first narrowed to an unresolved marker; folding records in (Q2) then gave it a
declaring origin instead, since records had to carry one and share the variant. See D2 (revised).

Every union a module can see resolves to a `Type::Union` carrying origin, and the two paths that
used to take a `Type::Named` and look it back up as a union by name (`resolve_contextual_name_in`
and `bare_form_hint`) were removed, so no union matches on spelling alone. Those lookups turned out
to be reachable only where they would have returned nothing anyway: `resolve_named_type` consults
the same `union_defs` map, so a name it left as `Type::Named` is a name that map does not hold.

### Q2: Records and components carry origin in this change, runtime included — settled in tasks 5.1 and 10.1

**Reversed after measuring.** Task 5.1 first deferred records to a follow-on. The reason recorded
was that giving them origin meant revisiting every `Type::Named` match site across seven crates —
which was the proposal's Impact list repeated as though it had been measured. It is 17 match sites
in four crates, and the deferral also understated the defect: the record hole is not a tidy scoping
line, it is live and silently corrupting. Two same-named records in different modules produced no
diagnostic and evaluated with a `string` in a field the declaring module typed `int` — byte for byte
the defect class this change closes for unions, in the nominal kind NX programs use most.

The real work was never the match sites. It is that record identity is a *subtype* relation:
`EffectiveRecordShape.ancestors` was a `Vec<Name>` walked in the asking module, so a foreign
record's `extends` clause was re-resolved against the consumer's declarations. Each link now
resolves in the module that wrote it and carries the declaration it reached, the same way D3 fixed
property type references one level down. `ResolvedRecordDefinition` already carried
`module_identity`; it was discarded at the comparison.

Components came with it. They share the `Type::Named` variant, their contract resolution mirrors
record resolution line for line, and a property can be typed by one — a probe confirmed the same
silent substitution. Fixing records while leaving components would have left one variant with two
identity rules. The peer namespace grew an element half (`ModuleNamespace`) so a component base
resolves where it was written, which is the only structural addition the component half needed.

The runtime half followed, in group 10. See D6.
