## Context

`update-records` and `property-references` give every record-shaped declaration two derived types,
`T.Update` and `T.Property`. For a record or an action they describe the fields a value of `T` is
written with. For a component they describe its `state` group instead, and a component without
state has neither. Inside a component body, a bare `Update` tag and a bare `Property` name are
shorthands for the component's own pair. Handlers patch state by returning `<Component>.Update`, and
both IR runtimes recognize a patch by the update record's target being the handler's owner
component.

The agreed design (thread decision, 2026-10-10) is:

- A component with state gets a derived record `T.State`, with `T.State.Property` and
  `T.State.Update`.
- A bare `State` inside a component replaces the bare `Update` and `Property` shorthands.

The first draft also derived a component's `T.Property` and `T.Update` from its props. Bret revised
that on 2026-10-12: for now, `T.Update` and `T.Property` exist only for records (and actions), and a
component has neither. A later change may give components and functions a derived props record,
perhaps `T.Properties`, with its own update record, so props can be built up and transformed.

AGENTS.md says no backward compatibility is needed yet, so the old spellings are removed rather
than kept as aliases.

## Goals / Non-Goals

**Goals:**

- One meaning for `T.Property` and `T.Update`: the fields of a record or action. A component has
  neither.
- A first-class record type for a component's state that works with the update intrinsics.
- One short spelling inside a component, `State.Update` and `State.Property`, in place of two
  special names.
- IR and both IR runtimes carry the state record explicitly, not by naming convention.

**Non-Goals:**

- Changing the host-owned state snapshot that runtime bindings pass in and out. It stays a field
  map keyed by state field name, with no `$type`.
- Typing the previewer's `setProps`, or any other host API, by a props patch. That can follow the
  future props record (perhaps `T.Properties.Update`), not `T.Update`, which a component no longer
  has.
- Letting a component change its own props. Props stay owned by the parent.
- Any derived type for a component's props, or a function's parameters. A props record and its
  update record are left for a later change.
- Anything in the HIR interpreter beyond keeping its existing tests passing, since it is being
  retired.

## Decisions

### `T.State` is a derived record with the state fields and no defaults

`T.State` has exactly the component's declared state fields, in declaration order, with their
declared types and optional marks. Abstract components can't declare state, so no fields are
inherited. Defaults are dropped, as they are for update records: a state default may read props,
and a `T.State` literal written outside the component has no props to read. So
`<Counter.State />` must give every field that isn't optional.

*Alternative considered:* keep defaults whose expressions don't read props. That makes whether a
field is required depend on the shape of its default, which is hard to explain and brittle under
edits.

`T.State` is a plain record in every other respect:

- It has `T.State.Update` and `T.State.Property`.
- It works with all four intrinsics.
- It is visible and importable wherever `T` is.
- It can't be extended, and nothing has a `.State` of its own except a component with state.

So `T.State.State`, `T.Update.State` and `Record.State` don't exist. A component without state has
no `T.State`, and naming it is a diagnostic that says so.

### A component has no `T.Update` or `T.Property`

A component is not a record, so it gets neither derived declaration, stateful or not, external or
not. Writing `Counter.Update` or `Counter.Property` is an unknown name, and the diagnostic suggests
`Counter.State.Update` or `Counter.State.Property` when the component has state. `Update` and
`Property` stay reserved as emitted-action names, so `<Component>.Update` and
`<Component>.Property` never name an action and stay free for a later props record.

*Alternative considered:* derive both from the component's props. Rejected for now (Bret,
2026-10-12): a props patch is better designed together with a props record for components and
functions, so it ships later as one feature.

### A bare `State` replaces the bare `Update` and `Property`

Inside a component body, `State` as a type reference, an element or record-literal tag, or the
first segment of a member access denotes `<Component>.State`. That covers its state defaults, its
rendered body, and the handlers bound in it. Like the shorthands it replaces, it wins over any
declaration named `State`. In a component without state it is a diagnostic saying the component
declares no state.

Outside a component, `State` is an ordinary name. Repo tests that declare `type State = …` at top
level keep working.

`Update` and `Property` become ordinary names everywhere. To help migration, an unresolved bare
`Update` tag or `Property` reference inside a component body gets a diagnostic that suggests
`State.Update` or `State.Property`. Outside a component, the diagnostic still shows the qualified
`<Type>.Update` form.

*Alternative considered:* keep bare `Update` as a synonym for `State.Update`. Rejected because it
would mean a different thing from `Counter.Update` written out, which is the confusion this change
removes.

### Handler routing

Inside a component body, a handler result whose type is the enclosing component's `T.State.Update`
patches that component's state. Any other update record is rejected as before.

At the root, every value is still a host effect. The runtime routing rules don't change. Only the test for "this
is the owner's state patch" does.

### IR names the state record

- **Component layout:** the component gains a trailing optional reference to its state record,
  `<Component>.State`.
- **State patches:** a state patch's record is `<Component>.State.Update`, whose update target is
  the `<Component>.State` record. A runtime treats a handler result as a state patch when the
  record's update target is the owner component's state record.
- **When the records are emitted:** emitting `T.State.Update` also emits `T.State`. The state
  record is emitted only when the program references it, its update, or its property union, as
  for other derived declarations. A component whose state is never patched or named keeps the
  same IR, apart from the version.
- **Version:** the component layout changes, so the schema version moves from 5 to 6.

*Alternative considered:* recognize a patch by the name `<Component>.State.Update`. Rejected
because a runtime would have to parse names, and libraries can re-export components under other
names.

The state field list on the component stays where it is. The runtime still needs the defaults to
initialize state, and `T.State` has none.

### Wire forms and generated companions

- **Canonical value:** a state patch carries `$type` `<Component>.State.Update`, and a `T.State`
  value carries `$type` `<Component>.State`.
- **Host state snapshot:** `typegen`'s existing `<Name>_state` contract stays the plain snapshot
  shape with no `$type`. It is now emitted for every exported component with state, not only
  external ones, since every such component has a `T.State`.
- **New companions:** `<Name>_state_update` and `<Name>_state_property` describe `T.State.Update`
  and `T.State.Property`.
- **Removed companions:** a component has no `<Name>_update` or `<Name>_property`.

## Risks / Trade-offs

- **[Breaking spelling change across docs and samples]** → Every `.nx` file, conformance program
  and doc page that uses bare `Update` or `Property`, or a component's `.Update`, moves in this
  change. The unresolved-name diagnostic suggests `State.Update`, so leftovers are easy to spot.
- **[Hosts that held `<Component>.Update` patches]** → Only hosts that build patches by hand see the
  new `$type`. Runtime bindings take state snapshots, not patches, so the .NET and TypeScript
  bindings' public shapes only change in generated companions.
- **[A user type named `State` is hidden inside components]** → The same trade-off was accepted for
  `Update` and `Property`. The component's own state is the more common meaning there.
- **[IR schema bump]** → Images built before this change are refused with the version error, which
  is the intended behavior. Committed images such as the TypeScript runtime's `dist` and the
  prelude image get regenerated with the repo's tooling.

## Migration Plan

This lands in one PR with no compatibility layer. Each of these moves to the new spellings in the
same PR, and the conformance corpus pins the new behavior for both runtimes:

- examples
- website pages
- `docs/nx-ir-format.md`
- the playground's counter example
- the IR conformance programs
- .NET generated fixtures

Rolling back means reverting the PR.

## Open Questions

- Should `T.State` exist for an external component with state? The default here is yes: it matches
  `<Name>_state`, and hosts that hold such state get a named type for it.
- (Future) A derived props record for components and functions, perhaps `T.Properties`, with its
  own update record (perhaps `T.Properties.Update`), to build up and transform props. Its name, and
  whether a component's `T.Update` should come back as a shorthand for it, are for that change.
