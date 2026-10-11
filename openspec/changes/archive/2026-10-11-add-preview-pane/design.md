## Context

The NX Viewer and Previewer design splits the work into four lanes: the compiler's language service
and source tagging (`add-source-tree`, `add-value-origin`), the viewer (`add-viewer`), the headless
preview session (`add-preview-session`), and the playground workbench. The first three are merged.
This change is the workbench's second stage, mounting the preview session in the playground, with
the part of the preview lane's second stage that needs a page: the timeline strip, the state
inspector and inputs for actions.

The playground today:

- `App.tsx` lays out a source pane (Edit, or Read through `<nx-viewer>`) and an output pane, which
  shows the `NxValueText` that the wasm SDK's `evaluateNx()` returns, through `<nx-value>`.
- One worker answers every request on one channel: `evaluate` (build the source as
  `playground.nx` and evaluate `root`) and the language queries. `worker/session.ts` holds the host
  and replaces it after a trap. Edits are debounced by 350 milliseconds before `evaluate`.
- An `ActionHandler` has no NX spelling, so a component's output fails with
  `nx-text-unspellable` and the pane says the value cannot be shown.

`@nx-lang/previewer` takes a prepared program, which `programFromImages` links from the images
`generateNxIr({ debug: true })` emits, and keeps everything else: ticks, branches, dispatch by
handler pointer, reload, origins. Its output is canonical JSON. A probe of the components example
with a bound handler confirms the pieces fit: building the source and linking its images, the IR
runtime evaluates `root` to `{ "$type": "Counter", "start": 3 }`, a session of `Counter` with those
props renders `Row`, `Label` and `Button` records with `onTapped` as
`{ "$type": "ActionHandler", "action": "Button.Tapped", "token": "h1-1" }`, and
`dispatchHandler("/content/1/onTapped", { "$type": "Button.Tapped" })` moves `count` from 3 to 4.

## Goals / Non-Goals

**Goals:**

- A visitor writes a component and uses it on the same page: presses its buttons, sees its state,
  steps back, branches, and keeps going while they edit.
- One spelling of values across the page: the preview's output reads exactly as Output's does,
  handlers aside.
- The preview leads back to the source that drew each record.
- The page stays responsive whatever the visitor's handlers do.

**Non-Goals:**

- The pane shell (resizable, hideable panes, the chat rail and composer, tabs below 1,100 pixels)
  from lane 4's first stage.
- Saving, sharing or loading a run as a scenario, or a run in the address. The session can export
  one already; the page to name and share it can follow.
- Selection from the source or the reading view to the preview.
- A props form. No schema describes a component's props until `T.Property` means props for
  components, so props are written in the source as the element `root` returns.
- Viewer hints, and any change to the language, the IR runtimes or `@nx-lang/previewer`.

## Decisions

### The session lives in the worker

The worker already holds the compiler, and the IR runtime is plain TypeScript, so the session runs
beside them. A handler that loops runs off the main thread, under the runtime's limits, and the
page only receives snapshots. The main thread holds no tick, only what it draws.

The channel gains one call, `{ kind: "preview", op, ... }`, with these operations:

| Operation | Does |
| --- | --- |
| `start` | Starts a session for a component of the latest build, or for `root`'s component |
| `dispatch` | Dispatches `{ handler, action }` by pointer from the current tick |
| `goTo` | Makes a tick current |
| `reload` | Reloads the session with the latest build, and sets new props when `root`'s changed |
| `stop` | Drops the session |

Each answers with a snapshot: the components the source declares, the session's component, the
current path as `{ id, cause, branches }` steps, the current tick's id, its output and emitted
actions as `NxValueText`, its state as one `NxValueText` per field, the handlers in the output with
their action names and input schemas, the origins of the output's records as source spans, and
what the last reload did. Snapshots are built from the current tick only, so their size does not
grow with the run.

The worker keeps the artifact of its latest successful build. `evaluate` already builds the
source; it now keeps the artifact and disposes the one before, and a preview call uses it rather
than building again. On each edit the main thread sends `evaluate`, then `reload` when Preview is
showing, so one compile serves both. A replaced host loses the artifact but not the session, which
needs no wasm: the next build restores the artifact.

Alternative considered: run the session on the main thread. Rejected: a runaway handler would
freeze the page until the operation limit stopped it, and every tick would sit beside the editor.

### The SDK spells runtime values

The preview's output is canonical JSON, and Output's text comes from the Rust formatter behind
`evaluateNx()`. `formatValue(value)` converts the canonical value to the interpreter's value and
runs that same formatter and annotator, so the two panes cannot drift apart in spacing, escaping,
number spelling or line breaks, and nodes get declarations from the same program.

The conversion today refuses `ActionHandler`, because a handler cannot be turned back into a
callable value. For spelling it does not need to be callable: `formatValue` spells it as the
record the runtimes return, `<ActionHandler action="Button.Tapped" token="h1-1" />`, which is
true to what the host sees and gives the action's name. `evaluateNx()` keeps refusing, since an
interpreter handler has no token.

Canonical JSON loses one thing the formatter needs: a whole `float64` arrives as the same number as
an `int`. The formatter already knows each property's declared type, so a number is spelled by its
position's type, and `options.type` gives the type of the whole value when the host knows it. The
state has no record type around it, so `options.state` names the component instead: the state is
spelled one `name=value` per line in declaration order, each field read by its declared type and
declared at its state field. Emitted actions are records, whose fields declare their own types.

Each node gains `pointer`, the JSON pointer of its value in the canonical form, so a host can go
from what the visitor points at to the value: the previewer's dispatch and `originOf` both take
pointers. A property node carries its value's pointer, so the `onTapped` property and its
`ActionHandler` record share one. `evaluateNx()` fills pointers too, at no cost and for any host
that wants them. The module export is new, so the ABI version moves to 8.

Alternative considered: a TypeScript formatter in `@nx-lang/value-view` or the previewer. Rejected:
a second formatter of the same text, with no source of declarations, and parity tests to keep it
honest.

### Buttons on the value, not a separate list

`<nx-value>` takes an `actions` function and shows each action as a button at the end of its
node's first line, reporting presses with `nx-value-action`. The playground returns one action for
each `ActionHandler` record, labelled by the action's own name (`Tapped` for `Button.Tapped`), so
each button sits where the handler is, inside the element that binds it, and two buttons with the
same action are told apart by where they are. The element stays generic: it knows nothing of
handlers, only of nodes and labels.

### Props come from the source

Preview runs the component `root` returns, with that element's props, because the source is
already the place a visitor writes values, and the editor already completes, checks and explains
them. The worker evaluates `root` through the IR runtime (`evaluateFunction(program, "root")`); a
record whose `$type` names a component of the entry module is a component element, and its other
properties are the props. When the props `root` gives change between builds, `reload` follows the
reload with `setProps`, so the run keeps its state and gains a props step. When `root` names
another component than the session's, the session restarts for it.

When `root` returns no component element, the first non-external component runs with no props
given, and the picker offers the others. A component with a required prop then fails to start, and
the pane says to end the program with its element.

### Inputs for actions from their schema

An action with fields opens a form built from `typeSchema({ name }, { direction: "input" })`, the
schema a host would validate the action against: strings, integers, floats, booleans and constant
unions get their own inputs and anything else a JSON field. The form submits a canonical action,
so what the runtime receives is what an embedding host would send. The schema is fetched once per
action per build and sent with the snapshot.

### Navigation by origin

Each tick keeps the origin of every record of its output. The worker turns them into source spans
with lines and columns, and the page gives each record node of the preview's `NxValueText` its
origin as the node's declaration before handing it to `<nx-value>`. Clicking a type name then
leads to the expression that built the record, and the existing `nx-value-navigate` handling
selects it in either Edit or Read. A record with no origin keeps its declaration.

### Limits

Every call runs with the runtime's limits: `maxCallDepth` of 200, matching the wasm SDK's
evaluation, and operation, input and range budgets sized so the examples and the question-flow
program run with room to spare while a loop that never ends stops within a second on a laptop. The
session's `maxTicks` stays at its default of 1,000. Output, emitted actions and state values are
cut at 100,000 characters in the worker, as Output's text is.

### Hot reload notices

The pane says what a reload did only when it changed course: a replay ("Your edit changed the
state, so the run was replayed from the start", with where it stopped if it did), or a failure
(the output marked out of date, with the runtime's message). A kept state needs no notice: the
output changes and the timeline gains an edit step.

## Risks / Trade-offs

- [The worker answers one request at a time] → a long dispatch delays the next hover or completion
  by up to the operation limit's time. The limits are set to keep that short.
- [Spelling `ActionHandler` as a record] → it reads like a record the program declares. Its type
  name is fixed and its node has no declaration, so hover says what it is, and the button beside it
  makes its role plain.
- [Snapshots per call] → each step sends the current output again rather than a difference. With
  the output cut at 100,000 characters, a step costs at most one such clone.
- [Props only from the source] → previewing a component with other props means editing the final
  element. That is one line, and a props form can follow once a props schema exists.
- [Branch choice in a strip] → a deeply branched run is hard to read as a strip. The tree view of
  the design's later stage can replace it; the strip covers the common case of a few branches.
