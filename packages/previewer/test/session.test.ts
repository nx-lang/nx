import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { NxIrRuntimeError, type NxHostValue } from "@nx-lang/ir-runtime";

import { createPreviewSession, programFromImages, type PreviewSession } from "../src/index.js";
import { actionOf, entry, expected, handlerFor, images, lifecycle, program } from "./fixtures.js";

const flow = program();

function start(options = {}): PreviewSession {
  return createPreviewSession(flow, lifecycle.component, lifecycle.props, options);
}

/** Dispatches the lifecycle's batches from `from` up to, not including, `to`. */
function run(session: PreviewSession, to: number, from = 0): void {
  for (let batch = from; batch < to; batch += 1) {
    session.dispatch(lifecycle.batches[batch]!);
  }
}

/** The diagnostics a call fails with. */
function failure(call: () => unknown): NxIrRuntimeError {
  try {
    call();
  } catch (error) {
    assert.ok(error instanceof NxIrRuntimeError, `expected an NxIrRuntimeError, got ${String(error)}`);
    return error;
  }
  assert.fail("expected the call to fail");
}

describe("a session runs one component of a prepared program", () => {
  it("starts the question flow with the recorded first render", () => {
    const session = start();
    assert.equal(session.ticks.length, 1);
    assert.equal(session.current, session.ticks[0]);
    assert.deepEqual(session.current.rendered, expected.initial);
    assert.deepEqual(session.current.cause, { kind: "initial", props: lifecycle.props });
    assert.equal(session.current.parent, undefined);
    assert.deepEqual(session.path, [session.current]);
  });

  it("links a program from images and refuses a list without the entry", () => {
    const built = images();
    assert.ok(programFromImages(built, entry).componentEntrypoints.has("Flow"));
    const missing = failure(() => programFromImages(built.filter((image) => image.identity !== entry), entry));
    assert.equal(missing.diagnostics[0]?.code, "nx-preview-program");
    const twice = failure(() => programFromImages([...built, built[0]!], entry));
    assert.equal(twice.diagnostics[0]?.code, "nx-preview-program");
  });

  it("fails with the runtime's diagnostics for props the component rejects", () => {
    const error = failure(() => createPreviewSession(flow, "Flow", { respondent: 42 }));
    assert.ok(error.diagnostics.length > 0);
    assert.ok(error.diagnostics.every((diagnostic) => diagnostic.code.startsWith("nx-ir-")));
  });

  it("refuses a maxTicks that is not a positive integer", () => {
    assert.equal(failure(() => start({ maxTicks: 0 })).diagnostics[0]?.code, "nx-preview-options");
  });
});

describe("each tick keeps what made it and what it rendered", () => {
  it("holds the batch, the state, the output and the effects of the first answer", () => {
    const session = start();
    const first = session.current;
    const tick = session.dispatch(lifecycle.batches[0]!);
    assert.equal(session.current, tick);
    assert.equal(tick.parent, first);
    assert.deepEqual(tick.cause, { kind: "batch", batch: lifecycle.batches[0], handlers: ["/children/1/onTextAnswered"] });
    assert.equal(tick.state.step, 1);
    assert.deepEqual(tick.rendered, expected.batches[0]!.rendered);
    assert.deepEqual(tick.effects, expected.batches[0]!.effects);
    assert.deepEqual(session.path, [first, tick]);
  });

  it("never changes a tick", () => {
    const session = start();
    const tick = session.dispatch(lifecycle.batches[0]!);
    assert.ok(Object.isFrozen(tick));
    assert.throws(() => {
      (tick.state as { step: NxHostValue }).step = 7;
    }, TypeError);
    assert.throws(() => {
      (tick.cause as { kind: string }).kind = "props";
    }, TypeError);
    session.dispatch(lifecycle.batches[1]!);
    assert.equal(tick.state.step, 1);
  });

  it("gives every recorded output for the whole lifecycle", () => {
    const session = start();
    lifecycle.batches.forEach((batch, index) => {
      const tick = session.dispatch(batch);
      assert.deepEqual(tick.rendered, expected.batches[index]!.rendered, `batch ${index}`);
      assert.deepEqual(tick.effects, expected.batches[index]!.effects, `batch ${index}`);
    });
    assert.equal(session.path.length, lifecycle.batches.length + 1);
  });
});

describe("going back runs nothing, and dispatching from the past branches", () => {
  it("keeps the engineer continuation when the reviewer tries designer", () => {
    const session = start();
    run(session, 2);
    const beforeRole = session.current;
    const engineer = session.dispatch(lifecycle.batches[2]!);
    run(session, 5, 3);
    const engineerEnd = session.current;
    const count = session.ticks.length;

    session.goTo(beforeRole);
    assert.equal(session.ticks.length, count, "going back adds no tick");
    assert.equal(session.current, beforeRole);

    const pointer = handlerFor(beforeRole.rendered, "Step.ChoiceAnswered")!;
    const designer = session.dispatchHandler(pointer, {
      $type: "Step.ChoiceAnswered",
      answer: { $type: "ChoiceAnswer", questionId: "role", selected: "designer" },
    });
    assert.deepEqual(session.childrenOf(beforeRole), [engineer, designer]);
    assert.ok(session.ticks.includes(engineerEnd));
    assert.equal(engineerEnd.parent?.parent, engineer);
    assert.equal(session.path.at(-1), designer);
    assert.equal(designer.state.role, "designer");

    // Returning to the first continuation runs nothing either.
    session.goTo(engineerEnd);
    assert.equal(session.ticks.length, count + 1);
    assert.deepEqual(session.path.map((tick) => tick.id), [1, 2, 3, 4, 5, 6]);
  });

  it("refuses a tick of another session", () => {
    const other = start();
    const session = start();
    assert.equal(failure(() => session.goTo(other.current)).diagnostics[0]?.code, "nx-preview-tick");
  });

  it("drops the oldest ticks off the current path past maxTicks", () => {
    const session = start({ maxTicks: 3 });
    run(session, 2);
    const [, firstAnswer, secondAnswer] = session.ticks;
    session.goTo(firstAnswer!);
    run(session, 2, 1);
    // Four ticks were made: the first render, two answers, and a second take on the second.
    assert.equal(session.ticks.length, 3);
    assert.ok(!session.ticks.includes(secondAnswer!));
    assert.deepEqual(session.childrenOf(firstAnswer!).length, 1);
  });
});

describe("a host can dispatch by handler", () => {
  it("answers through the step's handler with the token it carries", () => {
    const session = start();
    const tick = session.dispatchHandler("/children/1/onTextAnswered", actionOf(0));
    assert.deepEqual(tick.cause, {
      kind: "batch",
      batch: [{ $type: "ActionHandlerInvocation", token: "h1-1", action: actionOf(0) }],
      handlers: ["/children/1/onTextAnswered"],
    });
    assert.deepEqual(tick.rendered, expected.batches[0]!.rendered);
  });

  it("mixes entries by token and by handler in one batch", () => {
    const session = start();
    run(session, 1);
    const tick = session.dispatch([{ handler: "/children/1/onTextAnswered", action: actionOf(1) }]);
    assert.deepEqual(tick.rendered, expected.batches[1]!.rendered);
  });

  it("fails naming both actions for the wrong action, and adds no tick", () => {
    const session = start();
    const before = session.current;
    const error = failure(() =>
      session.dispatchHandler("/children/1/onTextAnswered", {
        $type: "Step.ChoiceAnswered",
        answer: { $type: "ChoiceAnswer", questionId: "name", selected: "Ada" },
      }),
    );
    assert.match(error.message, /Step\.TextAnswered/);
    assert.match(error.message, /Step\.ChoiceAnswered/);
    assert.equal(session.ticks.length, 1);
    assert.equal(session.current, before);
  });

  it("fails for a pointer at no handler and for a token the output does not hold", () => {
    const session = start();
    for (const pointer of ["/children/0", "/children/9/onTextAnswered", "children/1", "/children/01/onTextAnswered"]) {
      assert.equal(failure(() => session.dispatchHandler(pointer, actionOf(0))).diagnostics[0]?.code, "nx-preview-handler");
    }
    const stale = { $type: "ActionHandlerInvocation", token: "h7-1", action: actionOf(0) };
    assert.equal(failure(() => session.dispatch([stale])).diagnostics[0]?.code, "nx-preview-handler");
    assert.equal(session.ticks.length, 1);
  });
});

describe("a failure adds no tick", () => {
  it("leaves the timeline as it was when a batch exceeds the operation budget", () => {
    // The first render costs 267 operations and the first answer 303.
    const session = start({ maxOperations: 300 });
    const before = session.current;
    const error = failure(() => session.dispatch(lifecycle.batches[0]!));
    assert.equal(error.diagnostics[0]?.code, "nx-ir-resource-limit");
    assert.equal(error.diagnostics[0]?.limit?.name, "maxOperations");
    assert.equal(session.current, before);
    assert.equal(session.ticks.length, 1);
  });

  it("leaves the timeline as it was when new props fail", () => {
    const session = start();
    failure(() => session.setProps({ respondent: 42 }));
    assert.equal(session.ticks.length, 1);
  });
});

describe("props can change without losing state", () => {
  it("keeps the name the first render took from the old respondent", () => {
    const session = start();
    const first = session.current;
    assert.equal(first.state.name, "friend");
    const tick = session.setProps({ respondent: "Grace" });
    assert.equal(tick.parent, first);
    assert.deepEqual(tick.cause, { kind: "props", props: { respondent: "Grace" } });
    assert.deepEqual(tick.props, { respondent: "Grace" });
    assert.deepEqual(tick.state, first.state);
    assert.equal(tick.state.name, "friend");
  });

  it("keeps the state on the fifth question", () => {
    const session = start();
    run(session, 4);
    const fifth = session.current;
    const tick = session.setProps({ respondent: "Grace" });
    assert.deepEqual(tick.state, fifth.state);
    // Later batches run on the new tick's instance, with the props it was given.
    const next = session.dispatchHandler(handlerFor(tick.rendered, "Step.IntegerAnswered")!, actionOf(4));
    assert.deepEqual(next.props, { respondent: "Grace" });
    assert.equal(next.state.teamSize, 24);
  });
});
