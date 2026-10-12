## Context

The viewer (`@nx-lang/viewer`, spec `viewer`) reads a source tree as cards, rows and sentences, using
only what the tree's declaration table says about each kind: its name, its properties in order, their
types and defaults. The NX Viewer Hints proposal
(https://claude.ai/artifact/9o4DL9vaJFKKV1WxTeethK) adds a way for authors and hosts to say more.
Bret settled its main choices on 2026-10-10: a standalone `viewer Target = <…Viewer …/>` module
member rather than annotations, the keyword `viewer` rather than `view`, records named `…Viewer`, a
standard library `@nx/viewer`, hints usable beside the types or from outside with one syntax, and
later hints merging into earlier ones rather than replacing them.

One part of the proposal no longer holds. It assumed `T.Property` would list a component's props,
and hint records name properties through `Target.Property`. In #57 Bret chose that a component is
not a record and has no `T.Property` or `T.Update`, and floated a props record for components and
functions as a later idea. Hints on components need some way to name props, so this change has to
pick one.

## Goals / Non-Goals

**Goals:**

- One hint syntax that works beside a declaration and in a separate module.
- Hints checked against their targets, so a misspelled property or case is a type error and a
  rename in a library breaks its hints loudly.
- Resolution done once, in the compiler, so every reader of the source tree sees the same result.
- A lossless reading: a hint can move a property into the folded details strip, never hide it.
- Hints a host supplies without the program importing them.

**Non-Goals:**

- Hints in the change view or the previewer's inputs panel. The proposal says they should read the
  same hints; that is later work once those views exist.
- Templates or computed text in hints ("Shown in *chips* layout").
- Locale-specific hint modules as a feature. A host can already name a module per locale; nothing in
  this change chooses one.
- A props record for functions.
- Generated host types for `T.Props`.

## Decisions

### A hint is a module member with a constant value

`viewer Target = Value` is a new `ModuleMember`. `viewer` is a keyword only as the first token of a
module member, the way `component` and `action` are, so existing names called `viewer` keep working.
A hint has no visibility and no name of its own: nothing refers to it, and the compiler gathers it.

The value is ordinary NX markup built from `@nx/viewer` records, which keeps the hint vocabulary
documented and versioned like any library and lets the type checker do the checking. Values are
limited to written-out constructions, string and boolean literals, and bare names that resolve to
properties or cases. Nothing in a hint needs computing, and keeping it constant lets the compiler
read hints without evaluating them, so viewer declarations never reach NX IR.

*Alternatives considered* (from the proposal): annotations on declarations and doc-comment tags only
work beside the declaration and are unchecked or a sub-language of their own; a `let` value of a hint
type carries no intent and can't be reported as a duplicate; a JSON or YAML side file is unchecked.

### `Target` is a type parameter that the declaration binds

Each hint record has a leading `Target:type` parameter, and fields such as `title?:Target.Property`
and `of:Target.Property` use it. A viewer declaration binds `Target` for every `@nx/viewer`
construction in its value that leaves it unset, so authors write `<ElementViewer title=label />` and
`<PropertyViewer of=required … />` without `Target=`. Outside a viewer declaration the ordinary rule
holds: a generic record binds every type parameter explicitly.

That needs one language addition: a generic record's field may be typed `P.Property` for its own type
parameter `P`. It is rigid inside the record and becomes `A.Property` when the record is applied with
`P=A`, so the existing contextual rule resolves `title=label` against `Choice.Property`. Arguments
without a property union are a diagnostic. This is limited to records' own type parameters; a
component's type parameter gets nothing new. A general rule for derived names of type parameters,
`P.Update` included, is not needed yet.

### Components name their props through a new `T.Props` record (default picked for review)

A hint on a component binds `Target` to the component's derived props record, `T.Props`, so its
property references check against `T.Props.Property`. `T.Props` is new in this change. It follows
`T.State` closely:

- one field per prop, inherited props first, each with its type, optional mark and `content`
  modifier;
- no defaults, for the reason `T.State` has none: a default may read other props, and a literal
  written outside the component has none to read;
- the usual `T.Props.Update` and `T.Props.Property`, so the update intrinsics work on props;
- the component's type parameters, as a generic record's `R.Update` carries its record's;
- every component has one, even with no props, so a hint never depends on whether a component
  happens to declare props;
- `Props` is reserved as an emitted-action name, as `State` is. There is no bare `Props` shorthand
  inside a component, so a prop named `Props` stays legal.

This is the first half of the props-record idea Bret raised in #57: it gives the record and its
update and property union, which hints need now. Generated host types for `T.Props` and a props
record for functions are left for that later change. Until then `typegen` reports an exported
declaration that refers to `T.Props` rather than emitting something it would later change.

*Alternatives considered:*

- **A viewer-only property union for components.** Inside a hint, `Target.Property` would list a
  component's props. It needs no new record, but it brings back the `T.Property`-on-a-component
  meaning that #57 removed, for one context only.
- **Name the record `T.Properties`.** Bret floated that name. `T.Props.Property` and
  `T.Props.Update` read better than `T.Properties.Property`, and "props" is the word the NX docs use.
  This is easy to rename if Bret prefers the long form.
- **Keep prop defaults in `T.Props`.** That would make `<Card.Props />` match `<Card />`, which helps
  the later props-building work. It would also make `T.Props` behave differently from `T.State`.
  Left for that change to decide.

### Resolution happens in the compiler and reaches readers through the source tree

The language service resolves hints when a `sourceTree` request carries the `viewers` option. It
attaches a `viewer` value to each declaration entry that has a hint, so the viewer reads finished
data and does no type lookups. For a record, action or component, the value's `properties` list
holds every property once, in its resolved order, with its resolved label, phrases, format and
placement. For a union it lists every case; for an alias it gives the format. The table also lists
unions and aliases that type a listed property, so a reader never needs a second query.

Resolution order is the proposal's: along inheritance, base first; and at each step, the declaring
module, then the other modules of the program, then the host's hint modules in the order named.
Later hints override attribute by attribute. `PropertyViewer`s and `CaseViewer`s merge by `of`, and
the later hint's property order comes first. One deviation: a base's `name` is not inherited. A base
called "Question" should not make every derived kind read "Question". Without its own name, a
derived kind reads its declaration name split into words, which is the unhinted default.

"The program" is the entry module and everything it imports, standard libraries included. A host
supplies further hint modules by identity in the request. They are analyzed like any module, so a
host hint against a renamed property fails to type-check.

### Duplicates

Two hints for one target in one module are an error at the second. Two hints for one target in two
program modules that don't declare it are also an error, since neither has a better claim to come
later. It is reported on each document whose program loads both, at the hint or at the import that
brings in the other, and neither hint applies. Declaring-module, program and host hints never
conflict, since they apply in order. Two host modules can both hint a target because the host chose
their order.

### What the viewer does with hints

- **Card parts:** the attributes bound to `title`, `subtitle`, `badge` and `image` move into those
  parts of the card and keep their keys.
- **Rows:** labels replace property names on rows. Boolean phrases (`whenTrue`/`whenFalse`) replace
  a row only when the matching phrase is set, so nothing turns blank.
- **Details:** `place=details` moves a row or ghost into a details strip at the foot of the card.
  The lossless requirement now allows that strip beside the import strip.
- **Layouts:** `row` and `inline` put everything except the heading, title, subtitle, badge and
  image into the details strip.
- **Cases:** case labels show in the pill, with the case name and example in the hover.
- **Formats:** these change how a string literal shows, never what it is. `image` loads a picture
  only for `http:` and `https:` URLs, and a host can turn image loading off. A reading of untrusted
  source should not have to fetch from arbitrary hosts.

### What is left out of version one

- **Generic targets** (a generic record, component or union) are rejected. Binding `Target` to a
  generic declaration would need its type arguments, and hints don't depend on them; this can be
  allowed later by binding `Target` with its parameters unbound.
- **Unions with payload cases** are rejected as `UnionViewer` targets, because `CaseViewer`'s
  `of:Target` names a value, and a payload case can't be named without a payload.
- **`ElementViewer.key`**, which the proposal included for the change view, waits for that view.
- **`date`, `time` and `currency` formats** are dropped from `ValueFormat`. The viewer shows source,
  and formatting a written date or amount needs a locale; they can return with locale support.

## Risks / Trade-offs

- **`T.Props` is a language addition made for hints.** It is designed to stand on its own, to the
  same rules as `T.State`, so the later props work extends it rather than replacing it. The risk is
  that the later work wants defaults or a different name; both are noted for review.
- **Hints add analysis work to every document.** Resolution walks the program's modules and their
  hints only when a client asks with `viewers`, and duplicate detection reuses the module list that
  analysis already builds. Hint type-checking happens with the module's other members.
- **Cross-module duplicate reporting can surprise.** A document can see an error caused by two
  modules it imports. The diagnostic names both hints so the fix is clear.
- **The source tree grows.** `viewer` values only appear when asked for, so existing clients see
  identical answers.

## Migration Plan

Nothing existing changes meaning. `viewer` stays an ordinary name outside a module member's first
token. `Props` becomes a reserved emitted-action name; no `.nx` file in the repository emits an action
named `Props`. The `@nx/viewer` library is new and `unstable`.

## Open Questions

- **Props record (for Bret):** is `T.Props` right, with no defaults and no typegen yet? The
  alternatives are above.
- **Is a base's `name` not being inherited the right call?** The proposal's example sets `name` on
  every kind, so it does not settle this.
- **Should hint diagnostics from cross-module duplicates be warnings instead of errors?** An error
  matches the proposal. A warning would let a program build while two libraries disagree.
