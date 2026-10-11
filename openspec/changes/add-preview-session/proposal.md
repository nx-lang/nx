## Why

The viewer answers "does this say what I meant?". The previewer answers the reviewer's other
question, "does it behave right?": click through the survey as a designer with a team of 24, land
on the questions you expect, then go back and try "engineer". Today the only way to run a
component's lifecycle is to write a `program.json` by hand, as the conformance tests do, or to build
a host from the IR runtime's functions.

The IR runtime already has the right shape for a previewer. A host initializes a component, keeps
the opaque instance and state it returns, and sends batches of actions back with it. An instance is
never modified, so the one a dispatch was given stays valid. Value origins (`add-value-origin`) tell
the host which element expression built each record of the output. A previewer is mostly a matter
of keeping those snapshots and batches around, which is what the NX Viewer and Previewer design
calls the preview session (lane 3, stage 1).

## What Changes

- A new framework-free package, `@nx-lang/preview`, runs one component of a prepared program as a
  headless preview session on `@nx-lang/ir-runtime`. It has no UI; the playground and ReachMe build
  theirs on it.
- The session keeps a timeline. Each tick holds the batch or the props that produced it, the state,
  the rendered output with its origins, and the effects the handlers returned. A host can go back to
  any tick, and dispatching from an earlier tick starts a branch, while the old continuation is kept
  so the host can return to it.
- The session dispatches by handler, not only by token. A host names the handler record it read in
  the rendered output, and the session finds its current token, so a host never has to track the
  runtime's token numbering.
- A run's path saves as a scenario in the lifecycle format of `program.json`, with a name and, for
  each invocation, where its handler sat in the output, so a scenario can be replayed after the
  program changes. Replay stops at the first step whose handler is gone and says which.
- Hot reload: given a new program, the session keeps the current state when the new program accepts
  it, replays the path otherwise, and reports which it did and where a replay stopped.
- Every call runs with an origins report, so each tick knows which source span built each record of
  its output, for linking a preview to the viewer later.

The timeline strip, state inspector and inputs panel (stage 2), jump to step, coverage, and agent
test runs with stubbed tools (stage 3) are not part of this change. Nor is the playground: wiring a
preview pane into it belongs to the workbench lane.

## Capabilities

### New Capabilities

- `preview-session`: the headless preview session, its timeline and branches, dispatch by handler,
  scenarios, hot reload and origins.

### Modified Capabilities

None.

## Impact

- `packages/preview`: the new package and its tests, which run the question-flow conformance
  program's lifecycle through a session and compare with its expected results and origins.
- `docs/deployment.md` and `docs/deployment-setup.md`: the new package is published with the
  others.
- No change to the compiler, the IR format, either runtime or the wasm ABI.
