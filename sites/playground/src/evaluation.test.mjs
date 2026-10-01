/**
 * The output pane's state: what each answer does to it, and what the pane says without a value.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { initialEvaluation, outputNotice, reduceEvaluation } from "./evaluation.ts";

const error = (message) => ({ severity: "error", message, origin: "source", span: null });
const value = (text) => ({ kind: "value", value: { text, nodes: [] }, truncated: false });

const answer = (state, source, result) =>
  reduceEvaluation(reduceEvaluation(state, { kind: "started" }), { kind: "answered", source, result });

test("before the first answer, the pane says it is evaluating", () => {
  assert.equal(outputNotice(initialEvaluation), "evaluating");
});

test("a first source that does not compile says so, rather than evaluating forever", () => {
  const state = answer(initialEvaluation, "broken", { diagnostics: [error("Syntax error")], outcome: null });
  assert.equal(state.evaluating, false);
  assert.equal(state.outcome, null);
  assert.equal(outputNotice(state), "doesNotCompile");
  assert.equal(state.diagnosticsSource, "broken");
});

test("a value is shown, and a broken edit after it keeps it, marked stale", () => {
  const good = answer(initialEvaluation, "good", { diagnostics: [], outcome: value("42") });
  assert.equal(outputNotice(good), "none");
  assert.equal(good.outcomeSource, "good");

  const broken = answer(good, "good + ", { diagnostics: [error("Syntax error")], outcome: null });
  assert.equal(broken.stale, true);
  assert.deepEqual(broken.outcome, value("42"));
  assert.equal(broken.outcomeSource, "good", "hover still reads the source the value came from");
  assert.equal(broken.diagnosticsSource, "good + ", "markers are placed in the source that was checked");

  const fixed = answer(broken, "good", { diagnostics: [], outcome: value("42") });
  assert.equal(fixed.stale, false);
  assert.deepEqual(fixed.diagnostics, []);
});

test("a runtime error's diagnostics are the ones the editor marks", () => {
  const diagnostics = [error("Division by zero")];
  const state = answer(initialEvaluation, "1 / 0", { diagnostics: [], outcome: { kind: "error", diagnostics } });
  assert.deepEqual(state.diagnostics, diagnostics);
  assert.equal(outputNotice(state), "none");
});

test("a warning beside a value or a runtime error is marked too", () => {
  const warning = { severity: "warning", message: "Doc link `[Missing]` does not name a visible declaration", origin: "source", span: null };
  const withValue = answer(initialEvaluation, "good", { diagnostics: [warning], outcome: value("42") });
  assert.deepEqual(withValue.diagnostics, [warning]);
  assert.equal(outputNotice(withValue), "none");

  const runtime = [error("Division by zero")];
  const withError = answer(initialEvaluation, "1 / 0", { diagnostics: [warning], outcome: { kind: "error", diagnostics: runtime } });
  assert.deepEqual(withError.diagnostics, [warning, ...runtime]);
});

test("a compiler failure is reported, keeping the last value stale", () => {
  const good = answer(initialEvaluation, "good", { diagnostics: [], outcome: value("42") });
  const failed = reduceEvaluation(good, { kind: "failed", message: "The compiler crashed." });
  assert.equal(failed.failure, "The compiler crashed.");
  assert.equal(failed.stale, true);
  assert.equal(outputNotice(failed), "none");
  const firstFailed = reduceEvaluation(initialEvaluation, { kind: "failed", message: "Could not load." });
  assert.equal(outputNotice(firstFailed), "none");
});
