import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { createPreviewSession, type PreviewSession, type PreviewSessionOptions } from "../src/index.js";
import { failure, lifecycle, program, renamedRating, replaceOnce } from "./fixtures.js";

const flow = program();

function start(options: PreviewSessionOptions = {}): PreviewSession {
  return createPreviewSession(flow, lifecycle.component, lifecycle.props, options);
}

/** Answers the lifecycle's questions from the first up to, not including, `question`. */
function toQuestion(session: PreviewSession, question: number): PreviewSession {
  for (const batch of lifecycle.batches.slice(0, question - 1)) {
    session.dispatch(batch);
  }
  return session;
}

const labelFixed = (): ReturnType<typeof program> =>
  program((modules) =>
    replaceOnce(modules, "main.nx", "Attach your build configuration, if you can share it.", "Attach your CI configuration."),
  );

describe("the runtime's limits reach every call", () => {
  it("passes the budgets to the runtime, which refuses a value that is no budget", () => {
    for (const name of ["maxOperations", "maxInputSize"] as const) {
      assert.equal(failure(() => start({ [name]: -1 })).diagnostics[0]?.code, "nx-ir-options", name);
    }
  });

  it("stops a call at the range length", () => {
    const ranged = program((modules) =>
      replaceOnce(
        modules,
        "main.nx",
        "<Progress answered={answered} total={total} />",
        "<Progress answered={answered} total={total} />\n    {for i in 0..3 { <Progress answered={i} total={total} /> }}",
      ),
    );
    createPreviewSession(ranged, "Flow", lifecycle.props, { maxRangeLength: 3 });
    const error = failure(() => createPreviewSession(ranged, "Flow", lifecycle.props, { maxRangeLength: 2 }));
    assert.equal(error.diagnostics[0]?.limit?.name, "maxRangeLength");
  });

  it("stops a call at the call depth", () => {
    const error = failure(() => toQuestion(start({ maxCallDepth: 0 }), 31));
    assert.equal(error.diagnostics[0]?.limit?.name, "maxCallDepth");
  });

  it("refuses a batch larger than the input limit, and props as well", () => {
    // The props measure 2, and the first batch more than 5.
    const session = start({ maxInputSize: 5 });
    assert.equal(failure(() => session.dispatch(lifecycle.batches[0]!)).diagnostics[0]?.limit?.name, "maxInputSize");
    assert.equal(failure(() => session.setProps({ respondent: "x".repeat(1000) })).diagnostics[0]?.limit?.name, "maxInputSize");
    assert.equal(session.ticks.length, 1);
  });

  it("does not count the state the session passes back as input", () => {
    // Every batch fits in 20, but by question 22 the state measures 91.
    const session = toQuestion(start({ maxInputSize: 20 }), 22);
    assert.equal(session.setProps({ respondent: "Grace" }).cause.kind, "props");
    assert.equal(session.reload(labelFixed()).outcome, "kept");
  });
});

describe("a reload that reaches a limit", () => {
  it("keeps the old program rather than replaying", () => {
    // Every batch of the flow costs at most 707 operations. The edited title costs about 1,000 more
    // to build, so rendering it fails whatever the state.
    const session = toQuestion(start({ maxOperations: 800 }), 22);
    const before = session.current;
    const count = session.ticks.length;
    const heavy = program((modules) =>
      replaceOnce(modules, "main.nx", '<Screen title="Developer tooling survey"', `<Screen title={"Developer tooling survey" + "${" ".repeat(64_000)}"}`),
    );
    const result = session.reload(heavy);
    assert.ok(result.outcome === "failed");
    assert.equal(result.diagnostics[0]?.limit?.name, "maxOperations");
    assert.equal(session.current, before);
    assert.equal(session.ticks.length, count);
    assert.equal(session.program, flow);
  });
});

describe("maxTicks", () => {
  it("keeps a current path longer than the limit whole", () => {
    const session = toQuestion(start({ maxTicks: 3 }), 6);
    assert.equal(session.ticks.length, 6);
    assert.deepEqual(session.ticks, session.path);
  });

  it("keeps the path a reload started from while it replays", () => {
    const session = toQuestion(start({ maxTicks: 12 }), 10);
    const before = session.path;
    const result = session.reload(renamedRating());
    assert.ok(result.outcome === "replayed" && result.completed);
    for (const tick of before) {
      assert.ok(session.ticks.includes(tick), `tick ${tick.id} was dropped`);
    }
    // Once the replay is over, the limit applies again to what is off the current path.
    session.dispatch(lifecycle.batches[9]!);
    assert.equal(session.ticks.length, 12);
    assert.ok(session.path.every((tick) => session.ticks.includes(tick)));
  });
});
