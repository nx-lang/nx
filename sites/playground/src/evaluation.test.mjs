import { strict as assert } from "node:assert";
import { test } from "node:test";
import { initialPreview, reducePreview, callFor } from "./preview/model.ts";
import { outputEvaluation } from "./output/evaluation.ts";
import { outputErrorHeading, outputNotice } from "./evaluation.ts";

test("worker failures retain the last value and renderer, then reload the retained session", () => {
  const state = {
    source: "good",
    renderer: "component",
    diagnostics: [],
    problem: null,
    session: { output: { value: { text: "42", nodes: [] }, truncated: false } },
  };
  const good = { ...initialPreview, state, asked: "good" };
  const failed = reducePreview(good, {
    kind: "failed",
    message: "The compiler crashed.",
  });
  const output = outputEvaluation(failed, "broken");
  assert.equal(failed.state, state);
  assert.equal(output.outcome.value.text, "42");
  assert.equal(output.stale, true);
  assert.equal(output.failure, "The compiler crashed.");
  assert.equal(outputNotice(output), "none");
  assert.deepEqual(callFor(failed, true, "fixed"), {
    op: "reload",
    source: "fixed",
  });
});

test("pending component interactions are not stale", () => {
  const model = {
    ...initialPreview,
    asked: "good",
    state: { source: "good", diagnostics: [], session: null, problem: null },
    pending: { op: "goTo", tick: 1 },
  };
  assert.equal(outputEvaluation(model, "good").stale, false);
  assert.equal(outputEvaluation(model, "good").evaluating, true);
  assert.equal(outputEvaluation(model, "edited").stale, false);
  const failedEdit = {
    ...model,
    state: { ...model.state, diagnosticsSource: "edited" },
    asked: "edited",
    pending: null,
  };
  assert.equal(outputEvaluation(failedEdit, "edited").stale, true);
  const typing = {
    ...failedEdit,
    asked: "still broken",
    pending: { op: "reload", source: "still broken" },
  };
  assert.equal(outputEvaluation(typing, "still broken").stale, true);
  assert.equal(outputEvaluation(typing, "still broken").evaluating, true);
});

test("opening a failing document clears the previous session before the worker answers", () => {
  const old = {
    ...initialPreview,
    state: { source: "old", renderer: "component", session: {} },
  };
  const loading = reducePreview(old, {
    kind: "asked",
    call: { op: "start", source: "new", reset: true },
  });
  const failed = reducePreview(loading, {
    kind: "failed",
    message: "Worker stopped",
  });
  assert.equal(failed.state, null);
  assert.equal(failed.notice, null);
  assert.equal(outputEvaluation(failed, "new").outcome, null);
});

test("reload carries a picked component for a replacement worker", () => {
  const old = {
    ...initialPreview,
    picked: "B",
    state: { source: "old" },
    asked: "old",
  };
  assert.deepEqual(callFor(old, true, "new"), {
    op: "reload",
    source: "new",
    component: "B",
  });
});

test("formatting and evaluation headings stay consistent across presentations", () => {
  assert.equal(
    outputErrorHeading([{ code: "nx-output-format" }]),
    "The value could not be displayed",
  );
  assert.equal(
    outputErrorHeading([{ code: "nx-text-unspellable" }]),
    "The value has no NX spelling",
  );
  assert.equal(
    outputErrorHeading([{ code: "nx-ir-runtime" }]),
    "Evaluating root failed",
  );
});

test("late action replies and failures cannot restore a document after reset", () => {
  const old = {
    ...initialPreview,
    state: { source: "old", session: {} },
    asked: "old",
  };
  const reset = reducePreview(old, {
    kind: "asked",
    call: { op: "start", source: "new", reset: true },
  });
  assert.equal(reset.epoch, 1);
  const reply = {
    kind: "answered",
    epoch: 0,
    call: { op: "dispatch", handler: "h", action: {} },
    state: old.state,
  };
  assert.equal(reducePreview(reset, reply), reset);
  const failed = reducePreview(reset, {
    kind: "failed",
    epoch: 1,
    message: "New document failed",
  });
  assert.equal(reducePreview(failed, reply), failed);
  assert.equal(
    reducePreview(failed, {
      kind: "failed",
      epoch: 0,
      message: "Old request failed",
    }),
    failed,
  );
  const fresh = reducePreview(reset, {
    kind: "answered",
    epoch: 1,
    call: { op: "start", source: "new" },
    state: { source: "new" },
  });
  assert.equal(reducePreview(fresh, reply), fresh);
});
