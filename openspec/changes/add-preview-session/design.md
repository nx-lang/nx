## Context

See proposal.md for why. What shapes the approach:

- The NX Viewer and Previewer design, lane 3, stage 1: a session engine with no UI, giving init,
  dispatch, a timeline of snapshots, back and branch, scenarios in the `program.json` lifecycle
  format and hot reload through the runtime, tested against the question-flow conformance program.
  The design's packaging rule applies: framework-free, so the playground and ReachMe embed the same
  code.
- `@nx-lang/ir-runtime` provides everything the session runs on. `initializeComponent` returns the
  rendered output, the state and an opaque instance. `dispatchComponentActions` takes an instance
  and a batch and returns a new instance, the next state, the rendered output and the handlers'
  effects. An instance is never modified, and the one a dispatch was given stays valid for a retry.
  Initializing with a `state` option validates it as a complete state for the component, which is
  how a host keeps state across new props. An `origins` report gives, for each record of a call's
  value, its JSON pointer and the source span of the element expression that built it.
- In a lifecycle render, each handler in the output is an `ActionHandler` record with an `action`
  name and a `token` such as `h3-1`. A batch entry is an `ActionHandlerInvocation` naming a token
  and carrying the action record. Tokens are numbered per render, in the order of a depth-first walk
  of the output.
- The question-flow conformance program drives the 30-question `Flow` through 30 batches in
  `program.json`, and `expected/results.json` and `expected/origins.json` hold the rendered output
  and origins of the initial render and of each batch.

## Goals / Non-Goals

**Goals:**

- A host can run a component, step back, try another answer and return to the first path, with no
  re-execution for going back.
- A run can be saved, named, shared and replayed, in a format the conformance tests already use.
- An edit to the program keeps the reviewer where they were whenever the program allows it.
- Every rendered record can be traced to the source that built it.
- No framework and no compiler dependency: the session takes a prepared program, so it runs in a
  browser, a worker, Node or ReachMe's host alike.

**Non-Goals:**

- Any UI: the timeline strip, the state inspector and the inputs panel come in stage 2, on top of
  this package.
- Jump to step by scenario replay, coverage recording, and agent test runs with traces and stubbed
  tools.
- The playground preview pane, which is the workbench lane's.
- Running a component tree whose children the host initializes separately. The session runs one
  component; its rendered output holds the descriptors of the components it renders, as the
  runtime returns them.

## Decisions

### Naming: the previewer, a preview, a session

As with the viewer and a view, the previewer is the tool (`@nx-lang/previewer`), a preview is what
it shows, and a session is one run of one component in it (`createPreviewSession`). The capability
is named for the session, which is all this change builds; the previewer's UI comes later in the
same package or beside it.

### A package of its own on the IR runtime

`@nx-lang/previewer` depends on `@nx-lang/ir-runtime` and nothing else. The session takes an
`NxPreparedProgram`, not source text, so compiling stays the host's: the playground builds images
with `@nx-lang/sdk-wasm`, and ReachMe may load images it built ahead of time. A helper,
`programFromImages(images, entry)`, prepares and links a list of `{ identity, bytes }` images, the
shape `generateNxIr` returns, since every host would otherwise write the same ten lines.

Alternatives considered: putting the session in `@nx-lang/ir-runtime`. The runtime's job is to
evaluate one call, and it has no notion of history; a session is a host-side policy over it.

### The timeline is a tree of ticks

Each tick holds what produced it (the initial props, a batch, new props, or a reload), the program
that rendered it, its props, the instance, the state, the rendered output, its origin entries and
the effects. Ticks are immutable and know their parent. The session has a current tick, and `path`
is the list of ticks from the current tick's root to the current one. A session can have several
roots: its first render, and the first tick of each replay.

A tick keeps its program because an instance belongs to the program that made it: after a reload,
going back to a tick from before it and dispatching runs the old program, which is what lets a
reviewer compare the two.

Going back moves the current tick and runs nothing, since an instance stays valid. Dispatching from
a tick that already has children adds another child: a branch. The old continuation stays in the
tree, so a host can return to it, and a tick's children are kept in the order they were made.

Alternatives considered: one linear list that drops the ticks after the current one on a new
dispatch, as an editor's undo stack does. Simpler, but the design's point is to compare "designer"
with "engineer" without losing either, and a tree costs little more.

### Dispatch by handler

A host can dispatch a batch exactly as the runtime takes it. It can also dispatch by handler: it
passes the JSON pointer of an `ActionHandler` record in the current tick's rendered output, which
is where a UI found the handler it is about to call, and the action record. The session reads the
token at that pointer, checks that the action's `$type` is the one the record names, and builds the
invocation. The tick records both the invocation and the pointer.

An entry by token gets a pointer too: the session finds the record that carries the token in the
current output, and fails the entry when there is none. So every invocation a session dispatched
has a pointer, whichever way the host gave it, and only an action the component emits, which runs
the handler a parent bound and has no record in the output, has none.

This keeps hosts from depending on token numbering, and it is what makes a scenario survive an
edit: a pointer such as `/children/1/onChoiceAnswered` still names the same handler after the
tokens are renumbered, as long as the output keeps its shape there.

### Scenarios are lifecycles with locators

A scenario is a `program.json` lifecycle (`module`, `component`, `props`, `batches`) with two
optional members: a `name`, and `handlers`, which holds for each entry of each batch the pointer of
the handler it invoked, or `null` for an entry dispatched by token. A conformance runner ignores the
extra members, and a lifecycle with neither member is a valid scenario.

`scenario()` exports the current path. A reload tick on it adds nothing, since it changes the
program and not the run. A props change cannot be said in a lifecycle, which has one set of props,
so a path that holds one fails to export rather than export a run that replays differently.

`replay(scenario)` starts a new root from the scenario's props and dispatches each batch in turn.
For an entry with a pointer, it reads the current token at that pointer; for one without, it uses
the recorded entry. A step whose pointer no longer names a handler for the same action, or whose
token the current output does not hold, stops the replay: the ticks before it stay, and the result
names the batch and the entry that could not be placed. A batch the runtime fails stops it the same
way, with the batch and the runtime's diagnostics. `replayScenario(program, scenario)` starts a
session that way, for a host that opens a shared scenario.

Because every recorded invocation has a pointer, a scenario exported across a reload still
replays, though its tokens after the reload are the reloaded render's: the pointers place them.

Alternatives considered: addressing a handler by the source span of the element that bound it,
through origins. Spans move with every edit above them, so a pointer into the output is the more
stable key for a preview, and origins remain available to map a tick back to source.

### Hot reload keeps state, then replays

`reload(program)` re-runs the component under the new program in this order:

1. Initialize with the current tick's props and state. The runtime validates the state as complete
   for the component, so a program whose state shape is unchanged keeps the reviewer exactly where
   they were. The result is a new tick, a child of the current one, marked as a reload.
2. If that fails but the current props render alone, replay the current path under the new program
   from a new root: its batches, and its props changes, which an internal replay can hold though a
   scenario cannot.
3. If the replay stops, keep the ticks it made and report the step it stopped at.

If the props themselves do not render under the new program, the session keeps the old program and
reports the failure, so an edit that breaks the props never loses the timeline. The old ticks
remain in the tree in every case, so a host can compare.

### Origins on every call

Every call the session makes passes an origins report, so each tick holds its origin entries beside
its output, and `originOf(tick, pointer)` finds the record's span. The cost is the runtime's
bookkeeping for origins, which the value-origin change kept proportional to the records built. A
host that wants no origins can turn them off with an option.

### Failures leave the timeline as it was

A dispatch, a props change or an initialization that fails returns the runtime's diagnostics (an
`NxIrRuntimeError`'s, or a resource limit's) and adds no tick. Runtime limits such as
`maxOperations` and `maxInputSize` are session options passed to every call, since a preview runs
code a model wrote.

## Risks / Trade-offs

- [Memory grows with the tree] → Each tick holds a rendered output and a state. A survey of 30
  questions is small, but a long session of a large UI is not. Mitigation: a `maxTicks` option drops
  the oldest ticks that are not on the current path and have no children, so every kept tick keeps
  its parent, and the default is generous (1,000). A current path longer than the limit is kept
  whole.
- [A pointer can name a different handler after an edit] → If an edit moves a handler for the same
  action into the pointer's place, replay dispatches to it. The action type check catches most
  cases, and a stage 2 timeline can show a reloaded tick beside the original for the reviewer to
  compare.
- [The session runs one component] → A UI whose host initializes child components separately is
  not covered. The conformance programs, the question-flow `Flow` and the playground's examples
  render children as part of one component's output, which is the case this change serves.
