import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { NxIrRuntimeError } from "@nx-lang/ir-runtime";

import { createPreviewSession, replayScenario, type PreviewScenario, type PreviewSession } from "../src/index.js";
import { actionOf, expected, handlerFor, lifecycle, program, replaceOnce } from "./fixtures.js";

const flow = program();

/** Answers as a designer with a team of 24, by handler: name, email, role, design tool and team size. */
function designer(session: PreviewSession): void {
  const answers = [
    actionOf(0),
    actionOf(1),
    { $type: "Step.ChoiceAnswered", answer: { $type: "ChoiceAnswer", questionId: "role", selected: "designer" } },
    { $type: "Step.ChoiceAnswered", answer: { $type: "ChoiceAnswer", questionId: "designTool", selected: "canvas" } },
    actionOf(4),
  ];
  for (const action of answers) {
    session.dispatchHandler(handlerFor(session.current.rendered, action.$type)!, action);
  }
}

function failure(call: () => unknown): NxIrRuntimeError {
  try {
    call();
  } catch (error) {
    assert.ok(error instanceof NxIrRuntimeError, `expected an NxIrRuntimeError, got ${String(error)}`);
    return error;
  }
  assert.fail("expected the call to fail");
}

describe("a run saves and replays as a scenario", () => {
  it("replays the conformance lifecycle to the recorded outputs", () => {
    const { session, result } = replayScenario(flow, lifecycle);
    assert.equal(result.completed, true);
    assert.equal(result.stopped, undefined);
    assert.equal(result.tick, session.current);
    const [first, ...rest] = session.path;
    assert.deepEqual(first!.rendered, expected.initial);
    assert.equal(rest.length, 30);
    rest.forEach((tick, index) => {
      assert.deepEqual(tick.rendered, expected.batches[index]!.rendered, `batch ${index}`);
      assert.deepEqual(tick.effects, expected.batches[index]!.effects, `batch ${index}`);
    });
  });

  it("round-trips a named run through a new session", () => {
    const session = createPreviewSession(flow, "Flow", lifecycle.props);
    designer(session);
    const scenario = session.scenario("designer, team of 24");
    assert.equal(scenario.name, "designer, team of 24");
    assert.equal(scenario.module, "main.nx");
    assert.equal(scenario.component, "Flow");
    assert.deepEqual(scenario.props, lifecycle.props);
    assert.equal(scenario.batches.length, 5);
    assert.deepEqual(scenario.handlers?.map((pointers) => pointers.length), [1, 1, 1, 1, 1]);
    // A scenario is plain data: it survives JSON.
    const shared = JSON.parse(JSON.stringify(scenario)) as PreviewScenario;

    const { session: replayed, result } = replayScenario(flow, shared);
    assert.equal(result.completed, true);
    assert.equal(replayed.path.length, session.path.length);
    replayed.path.forEach((tick, index) => {
      assert.deepEqual(tick.state, session.path[index]!.state);
      assert.deepEqual(tick.rendered, session.path[index]!.rendered);
    });
    assert.equal(replayed.current.state.role, "designer");
    assert.equal(replayed.current.state.teamSize, 24);
  });

  it("replays within a session as a new root", () => {
    const session = createPreviewSession(flow, "Flow", lifecycle.props);
    designer(session);
    const scenario = session.scenario();
    const before = session.ticks.length;
    const result = session.replay(scenario);
    assert.equal(result.completed, true);
    assert.equal(session.roots.length, 2);
    assert.equal(session.ticks.length, before * 2);
    assert.equal(session.path[0], session.roots[1]);
  });

  it("stops where an edited program renders no handler, keeping the ticks before it", () => {
    const session = createPreviewSession(flow, "Flow", lifecycle.props);
    designer(session);
    const scenario = session.scenario();
    // The role question is no longer shown at step 2.
    const edited = program((modules) => replaceOnce(modules, "main.nx", "if step == 2 {", "if step == 99 {"));
    const { session: replayed, result } = replayScenario(edited, scenario);
    assert.equal(result.completed, false);
    assert.equal(result.stopped?.batch, 2);
    assert.equal(result.stopped?.entry, 0);
    assert.equal(result.stopped?.diagnostics[0]?.code, "nx-preview-handler");
    assert.equal(replayed.path.length, 3);
    assert.equal(result.tick, replayed.current);
    assert.equal(replayed.current.state.step, 2);
  });

  it("stops at an entry whose token the output does not hold", () => {
    const stale: PreviewScenario = {
      module: "main.nx",
      component: "Flow",
      props: lifecycle.props,
      batches: [lifecycle.batches[0]!, lifecycle.batches[0]!],
    };
    const { result } = replayScenario(flow, stale);
    assert.equal(result.completed, false);
    assert.deepEqual([result.stopped?.batch, result.stopped?.entry], [1, 0]);
  });

  it("stops at a batch the runtime fails", () => {
    const { result } = replayScenario(flow, lifecycle, { maxOperations: 400 });
    assert.equal(result.completed, false);
    assert.equal(result.stopped?.batch, 2);
    assert.equal(result.stopped?.entry, undefined);
    assert.equal(result.stopped?.diagnostics[0]?.code, "nx-ir-resource-limit");
  });

  it("refuses a path that changed the props", () => {
    const session = createPreviewSession(flow, "Flow", lifecycle.props);
    session.setProps({ respondent: "Grace" });
    assert.equal(failure(() => session.scenario()).diagnostics[0]?.code, "nx-preview-scenario");
  });

  it("refuses a malformed scenario, or one for another component or module, before anything runs", () => {
    const session = createPreviewSession(flow, "Flow", lifecycle.props);
    const malformed: unknown[] = [
      null,
      { ...lifecycle, batches: [{}] },
      { ...lifecycle, handlers: [] },
      { ...lifecycle, module: 3 },
      { ...lifecycle, component: "Screen" },
      { ...lifecycle, module: "other.nx" },
    ];
    for (const scenario of malformed) {
      assert.equal(failure(() => session.replay(scenario as PreviewScenario)).diagnostics[0]?.code, "nx-preview-scenario");
    }
    assert.equal(session.ticks.length, 1);
    failure(() => session.replay({ ...lifecycle, props: { respondent: 42 } }));
    assert.equal(session.ticks.length, 1);
  });
});
