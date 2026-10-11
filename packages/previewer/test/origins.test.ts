import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { createPreviewSession, replayScenario } from "../src/index.js";
import { expectedOrigins, lifecycle, program, sources } from "./fixtures.js";

const flow = program();

describe("each tick knows where its records came from", () => {
  it("holds the recorded origins of the first render", () => {
    const session = createPreviewSession(flow, lifecycle.component, lifecycle.props);
    assert.deepEqual(session.current.origins, expectedOrigins.initial);
  });

  it("holds the recorded origins of every batch", () => {
    const { session } = replayScenario(flow, lifecycle);
    session.path.slice(1).forEach((tick, index) => {
      assert.deepEqual(tick.origins, expectedOrigins.batches[index], `batch ${index}`);
    });
  });

  it("finds the element expression of a step's question", () => {
    const session = createPreviewSession(flow, lifecycle.component, lifecycle.props);
    const origin = session.originOf(session.current, "/children/1/question");
    assert.ok(origin !== undefined);
    assert.equal(origin.module, "main.nx");
    const source = Buffer.from(sources().get("main.nx")!, "utf8");
    assert.match(source.subarray(origin.start, origin.end).toString("utf8"), /^<ShortText id="name"/);
    assert.equal(session.originOf(session.current, "/children/1/question/label"), undefined);
  });

  it("reports no origins when they are turned off", () => {
    const session = createPreviewSession(flow, lifecycle.component, lifecycle.props, { origins: false });
    const tick = session.dispatch(lifecycle.batches[0]!);
    assert.equal(session.ticks[0]!.origins, undefined);
    assert.equal(tick.origins, undefined);
    assert.equal(session.originOf(tick, "/children/1/question"), undefined);
  });
});
