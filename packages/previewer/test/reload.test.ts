import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { createPreviewSession, type PreviewSession } from "../src/index.js";
import { expected, lifecycle, program, replaceOnce } from "./fixtures.js";

const flow = program();

/** A session of the flow on question `question`, by the lifecycle's batches. */
function onQuestion(question: number): PreviewSession {
  const session = createPreviewSession(flow, lifecycle.component, lifecycle.props);
  for (const batch of lifecycle.batches.slice(0, question - 1)) {
    session.dispatch(batch);
  }
  return session;
}

/** The label of the question a tick's step shows. */
function label(rendered: unknown): unknown {
  return (rendered as { children: { question?: { label?: unknown } }[] }).children[1]?.question?.label;
}

describe("hot reload keeps the reviewer where they were", () => {
  it("keeps the state when a label is fixed on question 22", () => {
    const session = onQuestion(22);
    const before = session.current;
    assert.equal(label(before.rendered), "Attach your build configuration, if you can share it.");
    const fixed = program((modules) =>
      replaceOnce(modules, "main.nx", "Attach your build configuration, if you can share it.", "Attach your CI configuration."),
    );
    const result = session.reload(fixed);
    assert.equal(result.outcome, "kept");
    assert.ok(result.outcome === "kept");
    assert.equal(result.tick, session.current);
    assert.equal(result.tick.parent, before);
    assert.deepEqual(result.tick.cause, { kind: "reload" });
    assert.deepEqual(result.tick.state, before.state);
    assert.equal(label(result.tick.rendered), "Attach your CI configuration.");
    assert.equal(session.program, fixed);

    // The next answer runs under the new program.
    const next = session.dispatch([
      { handler: "/children/1/onAnswered", action: (lifecycle.batches[21]![0] as { action: { $type: string } }).action },
    ]);
    assert.equal(next.program, fixed);
    assert.equal(next.state.answered, 22);
  });

  it("replays the path when a state field is renamed on question 10", () => {
    const session = onQuestion(10);
    const before = session.current;
    const count = session.ticks.length;
    const renamed = program((modules) => {
      const source = modules.get("main.nx")!;
      const at = source.indexOf("component <Flow");
      modules.set("main.nx", source.slice(0, at) + source.slice(at).replace(/\brating\b/g, "score"));
    });
    const result = session.reload(renamed);
    assert.equal(result.outcome, "replayed");
    assert.ok(result.outcome === "replayed");
    assert.equal(result.completed, true);
    assert.equal(session.program, renamed);
    assert.equal(session.path.length, 10);
    assert.equal(session.path[0]!.parent, undefined);
    assert.equal(session.roots.length, 2);
    assert.equal(session.ticks.length, count * 2);
    assert.ok(session.ticks.includes(before));
    assert.equal(result.tick.state.score, 0);
    assert.equal(result.tick.state.rating, undefined);
    assert.deepEqual(result.tick.rendered, expected.batches[8]!.rendered);
  });

  it("replays through an earlier reload and a props change", () => {
    const session = onQuestion(3);
    session.reload(program((modules) => replaceOnce(modules, "main.nx", "Which describes your work best?", "What is your role?")));
    session.setProps({ respondent: "Grace" });
    session.dispatch([
      { handler: "/children/1/onChoiceAnswered", action: (lifecycle.batches[2]![0] as { action: { $type: string } }).action },
    ]);
    const renamed = program((modules) => {
      const source = modules.get("main.nx")!;
      const at = source.indexOf("component <Flow");
      modules.set("main.nx", source.slice(0, at) + source.slice(at).replace(/\brating\b/g, "score"));
    });
    const result = session.reload(renamed);
    assert.ok(result.outcome === "replayed");
    assert.equal(result.completed, true);
    assert.deepEqual(
      session.path.map((tick) => tick.cause.kind),
      ["initial", "batch", "batch", "props", "batch"],
    );
    assert.deepEqual(session.current.props, { respondent: "Grace" });
    assert.equal(session.current.state.role, "engineer");
  });

  it("keeps the old program and the current tick when the props no longer render", () => {
    const session = onQuestion(5);
    const before = session.current;
    const count = session.ticks.length;
    const required = program((modules) =>
      replaceOnce(modules, "main.nx", 'component <Flow respondent:string = "there"', 'component <Flow respondent:string = "there" team:string'),
    );
    const result = session.reload(required);
    assert.equal(result.outcome, "failed");
    assert.ok(result.outcome === "failed");
    assert.ok(result.diagnostics.length > 0);
    assert.equal(session.current, before);
    assert.equal(session.ticks.length, count);
    assert.equal(session.program, flow);
  });
});
