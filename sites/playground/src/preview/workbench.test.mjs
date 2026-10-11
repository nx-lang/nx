/**
 * The preview as the worker runs it, driven under Node through the worker's session with the module
 * the browser loads: evaluating keeps the build, and each preview call answers with what the page
 * draws.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, describe, test } from "node:test";
import { NxEvaluationError, createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { createNxSession } from "../worker/session.ts";
import { PREVIEW_LIMITS, PreviewWorkbench } from "./workbench.ts";

const module = await loadNxModule();
const session = createNxSession({ module });
after(() => session.dispose());

const counter = readFileSync(new URL("../examples/nx/counter.nx", import.meta.url), "utf8");
const encoder = new TextEncoder();

let id = 0;
const evaluate = (source) => session.answer({ kind: "evaluate", id: ++id, source });
const preview = (call) => session.answer({ kind: "preview", id: ++id, call });

/** Evaluates `source`, as the page does before it asks the preview to run it, then calls. */
async function run(call) {
  if (call.source !== undefined) {
    await evaluate(call.source);
  }
  return preview(call);
}

const labels = (state) => state.session.path.map((step) => step.label);
const text = (value) => value.value.text;
/** The handler of the button whose label is `label`. */
function handlerOf(state, label) {
  const output = text(state.session.output);
  const at = output.indexOf(`label="${label}"`);
  assert.notEqual(at, -1, `no button "${label}"`);
  const node = state.session.output.value.nodes.find(
    (candidate) => candidate.role === "record" && candidate.type === "ActionHandler" && candidate.start > at
  );
  return node.pointer;
}
const press = (state, label) =>
  preview({ op: "dispatch", handler: handlerOf(state, label), action: { $type: "Button.Tapped" } });

describe("a preview of the counter example", () => {
  test("runs the component root returns an element of, with that element's props", async () => {
    const state = await run({ op: "start", source: counter });
    assert.equal(state.problem, null);
    assert.deepEqual(state.components, ["Counter"]);
    assert.equal(state.component, "Counter");
    assert.equal(state.source, counter);
    assert.deepEqual(labels(state), ["Start"]);
    assert.equal(text(state.session.state), "count=3");
    assert.match(text(state.session.output), /<Label text="Count: 3" \/>/);
    assert.deepEqual(state.session.emitted, []);
    assert.equal(state.session.handlers.length, 3);
    assert.deepEqual(state.session.actions["Button.Tapped"], { type: "Button.Tapped", label: "Tapped", fields: [] });
  });

  test("declares each record built from source where its element expression is", async () => {
    const state = await run({ op: "start", source: counter });
    const { nodes } = state.session.output.value;
    const label = nodes.findIndex((node) => node.type === "Label");
    assert.ok(state.session.built.includes(label));
    const span = nodes[label].declaration;
    const bytes = encoder.encode(counter);
    assert.equal(new TextDecoder().decode(bytes.subarray(span.startByte, span.endByte)), '<Label text={"Count: " + count} />');
    assert.equal(counter.split("\n")[span.startLine - 1].slice(span.startColumn - 1, span.endColumn - 1), '<Label text={"Count: " + count} />');
  });

  test("dispatches by handler, and goes back without running anything", async () => {
    let state = await run({ op: "start", source: counter });
    const first = state.session.current;
    state = await press(state, "Add one");
    state = await press(state, "Add one");
    assert.deepEqual(labels(state), ["Start", "Tapped", "Tapped"]);
    assert.equal(text(state.session.state), "count=5");
    assert.match(text(state.session.output), /Count: 5/);

    state = await preview({ op: "goTo", tick: first });
    assert.equal(state.session.current, first);
    assert.equal(text(state.session.state), "count=3");
    assert.deepEqual(labels(state), ["Start"]);
    assert.notEqual(state.session.forward, null);
  });

  test("branches from an earlier step, and Forward follows the branch last left", async () => {
    let state = await run({ op: "start", source: counter });
    const first = state.session.current;
    state = await press(state, "Add one");
    const run1 = state.session.current;
    state = await preview({ op: "goTo", tick: first });
    state = await press(state, "Reset");
    const run2 = state.session.current;
    assert.equal(state.session.path[0].children.length, 2);

    state = await preview({ op: "goTo", tick: first });
    assert.equal(state.session.forward, run2, "the branch just left");
    state = await preview({ op: "goTo", tick: run1 });
    state = await preview({ op: "goTo", tick: first });
    assert.equal(state.session.forward, run1);
    state = await preview({ op: "goTo", tick: state.session.forward });
    assert.equal(text(state.session.state), "count=4");
  });

  test("lists what a step emitted to the host", async () => {
    let state = await run({ op: "start", source: counter });
    state = await press(state, "Finish");
    assert.deepEqual(state.session.emitted.map(text), ["<Counter.Finished total=3 />"]);
  });

  test("reloads an edit in place, keeping the state", async () => {
    let state = await run({ op: "start", source: counter });
    state = await press(state, "Add one");
    const edited = counter.replace('"Count: "', '"Total: "');
    state = await run({ op: "reload", source: edited });
    assert.equal(state.notice, null);
    assert.deepEqual(labels(state), ["Start", "Tapped", "Edit"]);
    assert.equal(text(state.session.state), "count=4");
    assert.match(text(state.session.output), /Total: 4/);
    assert.equal(state.source, edited);
  });

  test("follows new props in root's element", async () => {
    let state = await run({ op: "start", source: counter });
    state = await press(state, "Add one");
    state = await run({ op: "reload", source: counter.replace("<Counter start=3 />", "<Counter start=10 />") });
    assert.equal(labels(state).at(-1), "Props");
    assert.equal(text(state.session.state), "count=4");
    state = await press(state, "Reset");
    assert.equal(text(state.session.state), "count=10");
  });

  test("replays the run when the state no longer fits, and says so", async () => {
    let state = await run({ op: "start", source: counter });
    state = await press(state, "Add one");
    state = await run({ op: "reload", source: counter.replaceAll("count", "total") });
    assert.equal(state.notice.kind, "replayed");
    assert.equal(state.notice.completed, true);
    assert.deepEqual(labels(state), ["Start", "Tapped"]);
    assert.equal(text(state.session.state), "total=4");
  });

  test("keeps the session as it was when an edit does not compile", async () => {
    let state = await run({ op: "start", source: counter });
    state = await press(state, "Add one");
    const broken = `${counter}\nlet broken = `;
    assert.equal((await evaluate(broken)).outcome, null);
    state = await preview({ op: "reload", source: broken });
    assert.equal(state.problem, null);
    assert.equal(state.source, counter);
    assert.deepEqual(labels(state), ["Start", "Tapped"]);
  });
});

describe("recovering and remembering", () => {
  test("runs a source that imports a library", async () => {
    const source = `import "@nx/agent"
component <Limits /> = {
  state { steps:int = 6 }
  <AgentLimits maxSteps={steps} />
}
`;
    const state = await run({ op: "start", source });
    assert.equal(state.problem, null, JSON.stringify(state.problem));
    assert.equal(text(state.session.output), "<AgentLimits maxSteps=6 />");
  });

  test("keeps showing the session after its host's builds are lost, and starts in a fresh worker", () => {
    const host = createNxHost(module);
    const compile = (source) => {
      try {
        return host.buildProgramArtifact(source, { fileName: "playground.nx" });
      } catch (error) {
        if (error instanceof NxEvaluationError) {
          return undefined;
        }
        throw error;
      }
    };
    try {
      // A fresh worker: asked to start before any evaluation, it builds the source itself.
      const workbench = new PreviewWorkbench(compile);
      let state = workbench.answer({ op: "start", source: counter });
      assert.equal(state.problem, null, JSON.stringify(state.problem));
      const first = state.session.current;
      state = workbench.answer({ op: "dispatch", handler: handlerOf(state, "Add one"), action: { $type: "Button.Tapped" } });

      // The host crashed: its builds are gone, the session is not.
      workbench.forgetBuild();
      state = workbench.answer({ op: "goTo", tick: first });
      assert.notEqual(state.session, null);
      assert.equal(text(state.session.state), "count=3");
      assert.equal(state.source, counter);
      workbench.dispose();
    } finally {
      host.dispose();
    }
  });

  test("tries props that failed again on every reload, and says so each time", async () => {
    const divides = counter.replace('"Count: " + count', '"Share: " + 60 / start');
    let state = await run({ op: "start", source: divides });
    assert.equal(state.problem, null, JSON.stringify(state.problem));
    state = await run({ op: "reload", source: divides.replace("start=3", "start=0") });
    assert.equal(state.notice?.kind, "failed");
    state = await run({ op: "reload", source: divides.replace("start=3", "start=0").replace("Share", "Part") });
    assert.equal(state.notice?.kind, "failed", "the props are tried again");
    state = await run({ op: "reload", source: divides.replace("start=3", "start=4") });
    assert.equal(state.notice, null);
    assert.equal(labels(state).at(-1), "Props");
    assert.match(text(state.session.output), /Share: 15/);
  });

  test("Forward remembers the branch left, whatever ran since", async () => {
    let state = await run({ op: "start", source: counter });
    const first = state.session.current;
    state = await press(state, "Add one");
    const second = state.session.current;
    state = await press(state, "Add one");
    const third = state.session.current;
    state = await preview({ op: "goTo", tick: second });
    state = await press(state, "Reset");
    const newest = state.session.current;
    // Back from the third step to the first, leaving step 2 through step 3.
    state = await preview({ op: "goTo", tick: third });
    state = await preview({ op: "goTo", tick: first });
    // A dispatch elsewhere does not forget it.
    state = await press(state, "Reset");
    state = await preview({ op: "goTo", tick: second });
    assert.deepEqual(state.session.path.at(-1).children.map((child) => child.id), [third, newest]);
    assert.equal(state.session.forward, third);
  });
});

describe("choosing what to preview", () => {
  const two = `external component <Button label:string emits { Tapped { } } />
component <Counter start:int = 0 /> = {
  state { count:int = {start} }
  <Button label={"Count " + count} onTapped=<Update count={count + 1} /> />
}
component <Stepper /> = {
  state { step:int = 1 }
  <Label text={"Step " + step} />
}
component <Needy name:string /> = { <Label text={name} /> }
`;

  test("runs the first component with no props when root names none", async () => {
    const state = await run({ op: "start", source: two });
    assert.deepEqual(state.components, ["Counter", "Stepper", "Needy"]);
    assert.equal(state.component, "Counter");
    assert.equal(text(state.session.state), "count=0");
  });

  test("starts a new session for the component picked", async () => {
    const state = await run({ op: "start", source: two, component: "Stepper" });
    assert.equal(state.component, "Stepper");
    assert.deepEqual(labels(state), ["Start"]);
    assert.match(text(state.session.output), /Step 1/);
  });

  test("says when the component needs props nothing gives", async () => {
    const state = await run({ op: "start", source: two, component: "Needy" });
    assert.equal(state.session, null);
    assert.equal(state.problem.kind, "startFailed");
    assert.equal(state.problem.missingProp, true);
    assert.match(state.problem.diagnostics[0].message, /name/);
  });

  test("says when there is no component, or only external ones", async () => {
    for (const source of ["let root() = 1", "external component <Button label:string />\nlet root() = 1"]) {
      const state = await run({ op: "start", source });
      assert.deepEqual(state.problem, { kind: "noComponent" });
      assert.equal(state.session, null);
    }
  });

  test("says when the source has never compiled", async () => {
    const fresh = createNxSession({ module });
    try {
      const state = await fresh.answer({ kind: "preview", id: 1, call: { op: "start", source: "let x = " } });
      assert.deepEqual(state.problem, { kind: "doesNotCompile" });
    } finally {
      fresh.dispose();
    }
  });
});

describe("actions and limits", () => {
  test("describes an action's fields from its input schema", async () => {
    const source = `action Answered = { questionId:string value:int note?:string }
external component <Ask emits { Answered } />
component <Asker /> = {
  state { last:string = "" }
  <Ask onAnswered=<Update last={action.questionId} /> />
}
`;
    let state = await run({ op: "start", source });
    assert.deepEqual(state.session.actions.Answered.fields, [
      { name: "questionId", required: true, kind: "string" },
      { name: "value", required: true, kind: "integer" },
      { name: "note", required: false, kind: "string" }
    ]);
    state = await preview({ op: "dispatch", handler: "/onAnswered", action: { $type: "Answered", questionId: "q1", value: 3 } });
    assert.equal(text(state.session.state), 'last="q1"');
  });

  test("stops a runaway handler at the runtime's limits and adds no step", async () => {
    const source = `external component <Button label:string emits { Tapped { } } />
let spin(n:int):int = { spin(n + 1) }
let fib(n:int):int = { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
component <Looper /> = {
  state { n:int = 0 }
  <Row>
    <Button label="Deep" onTapped=<Update n={spin(0)} /> />
    <Button label="Long" onTapped=<Update n={fib(60)} /> />
  </Row>
}
`;
    let state = await run({ op: "start", source });
    assert.equal(state.problem, null, JSON.stringify(state.problem));
    for (const label of ["Deep", "Long"]) {
      const started = Date.now();
      const after = await press(state, label);
      assert.equal(after.problem.kind, "dispatchFailed");
      assert.equal(after.problem.diagnostics[0].code, "nx-ir-resource-limit");
      assert.deepEqual(labels(after), ["Start"]);
      assert.ok(Date.now() - started < 5000, `${label} took ${Date.now() - started}ms`);
    }
    assert.equal(PREVIEW_LIMITS.maxCallDepth, 200);
  });
});
