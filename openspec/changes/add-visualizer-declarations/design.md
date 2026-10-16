## Context

The viewer (`@nx-lang/viewer`, spec `viewer`) read a source tree as cards, rows and sentences when
this change began, using only what the tree's declaration table says about each kind: its name, its properties in order, their
types and defaults. The NX Viewer Hints proposal (https://claude.ai/artifact/9o4DL9vaJFKKV1WxTeethK)
adds a way for authors and hosts to say more. Bret settled its main choices on 2026-10-10: a
standalone `viewer Target = <…Viewer …/>` module member rather than annotations, the keyword `viewer`
rather than `view`, records named `…Viewer`, a standard library `@nx/viewer`, declarations usable
beside the types or from outside with one syntax, and later declarations merging into earlier ones
rather than replacing them. On 2026-10-15 he renamed the keyword and the records from `viewer` and
`…Viewer` to `visualizer` and `…Visualizer`, and the library to `@nx/visualizer`, so the declarations
no longer share a name with the `<nx-viewer>` tool that reads them, and match the playground's Visual
view, which will also edit.

One part of the proposal no longer holds. It assumed `T.Property` would list a component's props, and
visualizer records name properties through `Target.Property`. In #57 Bret chose that a component is not a
record and has no `T.Property` or `T.Update`, and floated a props record for components and functions
as a later idea. Visualizer declarations on components need some way to name props, so this change has to
pick one.

## Goals / Non-Goals

**Goals:**

- One visualizer declaration syntax that works beside a declaration and in a separate module.
- Visualizer declarations checked against their targets, so a misspelled property or case is a type error
  and a rename in a library breaks its visualizer declarations loudly.
- Resolution done once, in the compiler, so every reader of the source tree sees the same result.
- A lossless reading: a visualizer declaration can move a property into the folded details strip, never
  hide it.
- Visualizer declarations a host supplies without the program importing them.

**Non-Goals:**

- Visualizer declarations in the change view or the previewer's inputs panel. The proposal says they
  should read the same visualizer declarations; that is later work once those views exist.
- Templates or computed text in visualizer declarations ("Shown in *chips* layout").
- Locale-specific visualizer modules as a feature. A host can already name a module per locale; nothing in
  this change chooses one.
- A props record for functions.
- Generated host types for `T.Props`.

## Decisions

### A visualizer declaration is a module member with a constant value

`visualizer Target = Value` is a new `ModuleMember`. `visualizer` is a keyword only as the first token of a
module member, the way `component` and `action` are, so existing names called `visualizer` keep working. A
visualizer declaration has no visibility and no name of its own: nothing refers to it, and the compiler
gathers it.

The value is ordinary NX markup built from `@nx/visualizer` records, which keeps the visualizer vocabulary
documented and versioned like any library and lets the type checker do the checking. Values are
limited to written-out constructions, string and boolean literals, and bare names that resolve to
properties or cases. Nothing in a visualizer declaration needs computing, and keeping it constant lets the
compiler read visualizer declarations without evaluating them, so visualizer declarations never reach NX IR.

*Alternatives considered* (from the proposal): annotations on declarations and doc-comment tags only
work beside the declaration and are unchecked or a sub-language of their own; a `let` value of a
visualizer declaration type carries no intent and can't be reported as a duplicate; a JSON or YAML side
file is unchecked.

### `Target` is a type parameter that the declaration binds

Each visualizer record has a leading `Target:type` parameter, and fields such as `title?:Target.Property`
and `of:Target.Property` use it. A visualizer declaration binds `Target` for every `@nx/visualizer`
construction in its value that leaves it unset, so authors write `<ElementVisualizer title=label />` and
`<PropertyVisualizer of=required … />` without `Target=`. Outside a visualizer declaration the ordinary rule
holds: a generic record binds every type parameter explicitly.

That needs one language addition: a generic record's field may be typed `P.Property` for its own type
parameter `P`. It is rigid inside the record and becomes `A.Property` when the record is applied with
`P=A`, so the existing contextual rule resolves `title=label` against `Choice.Property`. Arguments
without a property union are a diagnostic. This is limited to records' own type parameters; a
component's type parameter gets nothing new. A general rule for derived names of type parameters,
`P.Update` included, is not needed yet.

### Components name their props through a new `T.Props` record

A visualizer declaration on a component binds `Target` to the component's derived props record, `T.Props`,
so its property references check against `T.Props.Property`. `T.Props` is new in this change. It
follows `T.State` closely:

- one field per prop, inherited props first, each with its type, optional mark and `content`
  modifier;
- no defaults, for the reason `T.State` has none: a default may read other props, and a literal
  written outside the component has none to read;
- the usual `T.Props.Update` and `T.Props.Property`, so the update intrinsics work on props;
- the component's type parameters, as a generic record's `R.Update` carries its record's;
- every component has one, even with no props, so a visualizer declaration never depends on whether a
  component happens to declare props;
- `Props` is reserved as an emitted-action name, as `State` is. There is no bare `Props` shorthand
  inside a component, so a prop named `Props` stays legal.

This is the first half of the props-record idea Bret raised in #57: it gives the record and its update
and property union, which visualizer declarations need now. Generated host types for `T.Props` and a props
record for functions are left for that later change. Until then `typegen` reports an exported
declaration that refers to `T.Props` rather than emitting something it would later change. Bret
settled the naming on 2026-10-12: `Props` for the record and `Property` kept for its property union,
as on every other record.

*Alternatives considered:*

- **A visualizer-only property union for components.** Inside a visualizer declaration, `Target.Property`
  would list a component's props. It needs no new record, but it brings back the
  `T.Property`-on-a-component meaning that #57 removed, for one context only.
- **Name the record `T.Properties`.** Bret floated that name, then chose `Props`: it is the word
  React, Vue, Svelte and the NX docs use, and `T.Props.Property` reads better than
  `T.Properties.Property`.
- **Rename `.Property` to `.Prop`.** Frameworks use "prop" only for component inputs, but every NX
  record derives `.Property` (`User.Property`), so Bret kept `.Property`.
- **Keep prop defaults in `T.Props`.** That would make `<Card.Props />` match `<Card />`, which helps
  the later props-building work. It would also make `T.Props` behave differently from `T.State`.
  Left for that change to decide.

### Resolution happens in the compiler and reaches readers through the source tree

The language service resolves visualizer declarations when a `sourceTree` request carries the `visualizers`
option. It attaches a `visualizer` value to each declaration entry that has a visualizer, so the viewer reads
finished data and does no type lookups. For a record, action or component, the value's `properties`
list holds every property once, in its resolved order, with its resolved label, phrases, format and
placement. For a union it lists every case; for an alias it gives the format. The table also lists
unions and aliases that type a listed property, so a reader never needs a second query.

Resolution order is the proposal's: along inheritance, base first; and at each step, the declaring
module, then the other modules of the program, then the host's visualizer modules in the order named.
Later visualizer declarations override attribute by attribute. `PropertyVisualizer`s merge by
`property`, and the later visualizer declaration's property order comes first. Every attribute is
inherited. (While `ElementVisualizer` had a `name`, one deviation applied: a base's `name` was not
inherited.) A base called "Question" should not make every derived kind read "Question". Without
its own name, a derived kind reads its declaration name split into words, which is the default
default.

"The program" is the entry module and everything it imports, standard libraries included. A host
supplies further visualizer modules by identity in the request. They are analyzed like any module, so a
host visualizer declaration against a renamed property fails to type-check.

### Duplicates

Two visualizer declarations for one target in one module are an error at the second. Two visualizer
declarations for one target in two program modules that don't declare it are also an error, since
neither has a better claim to come later. It is reported on each document whose program loads both, at
the visualizer declaration or at the import that brings in the other, and neither visualizer declaration
applies. Declaring-module, program and host visualizer declarations never conflict, since they apply in
order. Two host modules can both declare a visualizer for a target because the host chose their order.

### What the viewer does with visualizer declarations

The viewer's cards gave way, during this change, to a reading of the document's own text: an
element over several lines reads as a group with a row to each property, an element on one line as
its name and properties, and the rest as written, in the editor's colors. The reading uses:

- **Title:** the attribute bound to `title` reads as its value alone on an element's line.
- **Id:** the attribute bound to `id` reads as one quiet token, its value alone, after the element's
  name or, with `idPosition=end`, last on its line.
- **Icon:** `icon` names a Lucide icon an element starts with when it starts a line; a kind inherits it.
- **Formats:** `id` reads a string as a quiet token, and `color` puts a swatch before a string that
  names a color. A format changes how a string shows, never what it is.

On 2026-10-16 Bret cut the library down to what the reading uses: `ElementVisualizer` keeps
`title`, `id`, `idPosition` (now `start | end`) and `icon`; `PropertyVisualizer` keeps `format`, its
`of` renamed `property`; `ValueFormat` keeps `id` and `color`. `name`, `subtitle`, `badge`, `image`,
`layout`, labels, boolean phrases, places, `UnionVisualizer`, `CaseVisualizer` and the other formats
were removed, to return when the reading needs them, so a union cannot have a visualizer for now.

### What is left out of version one

- **Generic targets** (a generic record, component or union) are rejected. Binding `Target` to a
  generic declaration would need its type arguments, and visualizer declarations don't depend on them;
  this can be allowed later by binding `Target` with its parameters unbound.
- **Unions** are rejected as targets, since `UnionVisualizer` was removed. When it returns, a union
  with payload cases should still be rejected, because a case visualizer names a case by value, and a
  payload case can't be named without a payload.
- **`ElementVisualizer.key`**, which the proposal included for the change view, waits for that view.
- **`date`, `time` and `currency` formats** are dropped from `ValueFormat`. The viewer shows source,
  and formatting a written date or amount needs a locale; they can return with locale support.

## Risks / Trade-offs

- **`T.Props` is a language addition made for visualizer declarations.** It is designed to stand on its
  own, to the same rules as `T.State`, so the later props work extends it rather than replacing it.
  The risk is that the later work wants defaults; that would be a small change to this record.
- **Visualizer declarations add analysis work to every document.** Resolution walks the program's modules
  and their visualizer declarations only when a client asks with `visualizers`, and duplicate detection reuses
  the module list that analysis already builds. Visualizer declaration type-checking happens with the
  module's other members.
- **Cross-module duplicate reporting can surprise.** A document can see an error caused by two
  modules it imports. The diagnostic names both visualizer declarations so the fix is clear.
- **The source tree grows.** `visualizer` values only appear when asked for, so existing clients see
  identical answers.

## Migration Plan

Nothing existing changes meaning. `visualizer` stays an ordinary name outside a module member's first
token. `Props` becomes a reserved emitted-action name; no `.nx` file in the repository emits an action
named `Props`. The `@nx/visualizer` library is new and `unstable`.

## Open Questions

- **Is a base's `name` not being inherited the right call?** The proposal's example sets `name` on
  every kind, so it does not settle this.
- **Should visualizer diagnostics from cross-module duplicates be warnings instead of errors?** An error
  matches the proposal. A warning would let a program build while two libraries disagree.
