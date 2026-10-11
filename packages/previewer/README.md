# @nx-lang/previewer

> **Unstable.** The API and the scenario format may change incompatibly in any release, a patch
> release included. Pin an exact version.

Runs one NX component as a headless preview session on
[`@nx-lang/ir-runtime`](../../runtime/typescript). The viewer answers "does this say what I
meant?"; the previewer answers "does it behave right?": click through a survey as a designer with a
team of 24, step back, try "engineer", and keep both runs.

The package has no UI and no framework. A host draws the session's current output and its timeline
however it likes, in a browser, a worker or Node.

## What a session keeps

- **Ticks.** Every render is a tick: what made it (the first props, a batch, new props or a
  reload), the props, the state, the rendered output, the effects the handlers returned, and where
  each record of the output came from. A tick never changes.
- **A tree.** Going back to a tick runs nothing. Dispatching from a tick that already has children
  adds another child, so both continuations stay and the host can return to either.
- **Handlers by pointer.** A host dispatches to the `ActionHandler` record it found in the output,
  by its JSON pointer, and the session finds the token, so the host never tracks token numbering.
- **Scenarios.** The current path saves as a `program.json` lifecycle with a name and each entry's
  handler pointer, and replays in any session of the same component, after an edit too.
- **Hot reload.** Given a new program, the session keeps the state when the new program accepts it,
  replays the path when it does not, and keeps the old program when the props no longer render.

## Example

The session takes a prepared program, so compiling stays the host's. `programFromImages` links the
images `@nx-lang/sdk-wasm`'s `generateNxIr` emits, or images a host built ahead of time.

```ts
import type { NxHostRecord } from "@nx-lang/ir-runtime";
import { createPreviewSession, programFromImages, replayScenario, type PreviewImage } from "@nx-lang/previewer";

/** Runs the question flow as a designer, tries "engineer" instead, and replays the first run. */
export function previewFlow(images: readonly PreviewImage[]) {
  const program = programFromImages(images, "main.nx");
  const session = createPreviewSession(program, "Flow", { respondent: "friend" });

  // Answer through the handlers the output renders, by where they sit in it.
  const answer = (action: NxHostRecord & { $type: string }) =>
    session.dispatchHandler(`/children/1/on${action.$type.slice("Step.".length)}`, action);
  answer({ $type: "Step.TextAnswered", answer: { $type: "TextAnswer", questionId: "name", value: "Ada" } });
  answer({ $type: "Step.TextAnswered", answer: { $type: "TextAnswer", questionId: "email", value: "ada@example.com" } });
  const beforeRole = session.current;
  answer({ $type: "Step.ChoiceAnswered", answer: { $type: "ChoiceAnswer", questionId: "role", selected: "designer" } });
  answer({ $type: "Step.ChoiceAnswered", answer: { $type: "ChoiceAnswer", questionId: "designTool", selected: "canvas" } });
  answer({ $type: "Step.IntegerAnswered", answer: { $type: "IntegerAnswer", questionId: "teamSize", value: 24 } });

  // Save the run before branching: a scenario is the current path, and plain JSON.
  const shared = JSON.stringify(session.scenario("designer, team of 24"));

  // Go back and try another role. Nothing is re-run, and the designer run stays in the tree.
  session.goTo(beforeRole);
  answer({ $type: "Step.ChoiceAnswered", answer: { $type: "ChoiceAnswer", questionId: "role", selected: "engineer" } });
  const branches = session.childrenOf(beforeRole).map((tick) => tick.state.role);

  // Anyone with the same program can replay the saved run.
  const { session: replayed, result } = replayScenario(program, JSON.parse(shared));
  return { branches, completed: result.completed, teamSize: replayed.current.state.teamSize };
}
```

`previewFlow` returns `{ branches: ["designer", "engineer"], completed: true, teamSize: 24 }` for
the question-flow conformance program in `specs/ir-conformance/question-flow`.

## Dispatching

- `dispatch(batch)` takes a batch as the runtime does: `ActionHandlerInvocation` entries naming
  tokens, or actions the component emits. An entry may also be `{ handler, action }`, a handler's
  JSON pointer and the action for it.
- `dispatchHandler(pointer, action)` dispatches one action by pointer. The action's `$type` must be
  the action the handler record names.
- `setProps(props)` renders the current state with new props.

Each of these adds a child of the current tick and makes it current. A call that fails throws an
`NxIrRuntimeError` and adds no tick. Its diagnostics are the runtime's (`nx-ir-*`), or the
session's own:

- `nx-preview-handler`: an entry names no handler in the current output, a handler for another
  action, or a token the output does not hold;
- `nx-preview-batch`: a batch is not a list;
- `nx-preview-scenario`: a scenario is malformed or for another component or module, or a path
  with a props change is exported;
- `nx-preview-tick`: a tick is not one the session keeps;
- `nx-preview-options`: `maxTicks` is not a positive integer;
- `nx-preview-program`: `programFromImages` was given two images for one identity, or none for
  the entry.

`setProps` and a reload render again from the start, so the runtime numbers the new output's tokens
afresh (`h1-1`, …). Read a token from the current tick only, or dispatch by pointer, which needs no
token at all.

## Hot reload

`reload(program)` returns what it did:

- `kept`: the new program accepted the current props and state, and the output is re-rendered as a
  reload tick, a child of the current one.
- `replayed`: the state no longer fits, so the current path ran again under the new program, from a
  new root. `completed` and `stopped` say how far it got.
- `failed`: the props no longer render, or rendering reached one of the session's limits. The
  session keeps the old program and the current tick.

The reload itself drops no tick of the path it started from, whatever `maxTicks` says. After it,
`maxTicks` applies as usual. The old path is off the current one, and the session drops a tick
only once it has no children, so once the session is past the limit, later calls drop the old path
from its last tick backward, starting with the tick the reload started from. A host that compares
long runs side by side should raise `maxTicks` to fit both.

A tick keeps the program that rendered it: going back to a tick from before a reload and
dispatching runs the old program.

## Origins

Every call passes an origins report, so each tick's `origins` lists, for each record of its output,
the module and UTF-8 byte span of the element expression that built it. `originOf(tick, pointer)`
finds one record's. The images need their debug sections (`generateNxIr({ debug: true })`) for
records to have origins. Pass `{ origins: false }` to turn the reports off.

## Options

`createPreviewSession(program, component, props, options)` takes the runtime's limits,
`maxOperations`, `maxInputSize`, `maxCallDepth` and `maxRangeLength`, which apply to every call the
session makes; a preview runs code a model may have written, so set them. `maxInputSize` bounds what
the host passes, the props and the batches; the state that `setProps` and a reload pass back is the
runtime's own and is not measured. `maxTicks` (1,000 by default) bounds the tree: past it, the
oldest ticks off the current path that have no children are dropped, and a current path longer
than the limit is kept whole.
