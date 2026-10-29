/**
 * The page's side of the preview, under Node: the model's transitions, action forms and source
 * spans.
 */
import { strict as assert } from "node:assert";
import { describe, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { actionFrom, fieldsOf, initialInput } from "./fields.ts";
import { callFor, initialPreview, reducePreview, sessionLost } from "./model.ts";
import { SourceIndex } from "./spans.ts";

const running = (source) => ({
  source,
  components: ["Counter"],
  component: "Counter",
  problem: null,
  notice: null,
  session: { path: [], current: 1, forward: null }
});

describe("the preview model", () => {
  test("starts on the latest source that compiled, once the preview shows", () => {
    assert.equal(callFor(initialPreview, false, "a"), null);
    assert.equal(callFor(initialPreview, true, null), null);
    assert.deepEqual(callFor(initialPreview, true, "a"), { op: "start", source: "a" });
  });

  test("reloads a newer build, and asks nothing for the build it runs", () => {
    let model = reducePreview(initialPreview, { kind: "asked", call: { op: "start", source: "a" } });
    assert.equal(callFor(model, true, "a"), null);
    model = reducePreview(model, { kind: "answered", call: { op: "start", source: "a" }, state: running("a") });
    assert.deepEqual(callFor(model, true, "b"), { op: "reload", source: "b" });
  });

  test("reloads the workbench when a component was picked but no session runs", () => {
    let model = reducePreview(initialPreview, { kind: "asked", call: { op: "start", source: "a", component: "Stepper" } });
    model = reducePreview(model, {
      kind: "answered",
      call: { op: "start", source: "a", component: "Stepper" },
      state: { ...running("a"), session: null, problem: { kind: "noComponent" } }
    });
    assert.deepEqual(callFor(model, true, "b"), { op: "reload", source: "b", component: "Stepper" });
  });

  test("keeps a reload's notice until the next start or reload", () => {
    const notice = { kind: "failed", diagnostics: [] };
    let model = reducePreview(initialPreview, { kind: "answered", call: { op: "reload", source: "a" }, state: { ...running("a"), notice } });
    assert.equal(model.notice, notice);
    model = reducePreview(model, { kind: "answered", call: { op: "goTo", tick: 1 }, state: running("a") });
    assert.equal(model.notice, notice);
    model = reducePreview(model, { kind: "answered", call: { op: "reload", source: "b" }, state: running("b") });
    assert.equal(model.notice, null);
  });

  test("after the worker failed, waits for a new build rather than asking again", () => {
    let model = reducePreview(initialPreview, { kind: "asked", call: { op: "start", source: "a" } });
    model = reducePreview(model, { kind: "failed", message: "The compiler crashed." });
    assert.equal(model.failure, "The compiler crashed.");
    assert.equal(model.state, null);
    assert.equal(callFor(model, true, "a"), null);
    assert.deepEqual(callFor(model, true, "b"), { op: "start", source: "b" });
  });

  test("tells a session the worker lost from one that never started", () => {
    const lost = { ...running("a"), session: null };
    assert.equal(sessionLost({ op: "goTo", tick: 2 }, running("a"), lost), true);
    assert.equal(sessionLost({ op: "start", source: "a" }, running("a"), lost), false);
    assert.equal(sessionLost({ op: "goTo", tick: 2 }, null, lost), false);
    assert.equal(sessionLost({ op: "dispatch", handler: "", action: {} }, running("a"), { ...lost, problem: { kind: "noComponent" } }), false);
  });
});

const module = await loadNxModule();

describe("action forms", () => {
  const host = createNxHost(module);
  const artifact = host.buildProgramArtifact(
    `type Kind = a | b
action Answered = { questionId:string value:int ratio:float64 = 0.5 done:boolean = false kind:Kind = a tags?:string+ }
`,
    { fileName: "playground.nx" }
  );
  const fields = fieldsOf(artifact.typeSchema({ name: "Answered" }, { direction: "input" }));
  artifact.dispose();
  host.dispose();

  test("gives each field an input of its kind, starting from its default", () => {
    assert.deepEqual(fields, [
      { name: "questionId", required: true, kind: "string" },
      { name: "value", required: true, kind: "integer" },
      { name: "ratio", required: false, kind: "number", default: 0.5 },
      { name: "done", required: false, kind: "boolean", default: false },
      { name: "kind", required: false, kind: "cases", cases: ["a", "b"], default: "a" },
      { name: "tags", required: false, kind: "json" }
    ]);
    assert.deepEqual(fields.map(initialInput), ["", "", "0.5", false, "a", ""]);
  });

  test("makes the action from what the form holds, leaving out empty optional fields", () => {
    const made = actionFrom("Answered", fields, { questionId: "q1", value: "3", ratio: "", tags: '["x"]', done: true });
    assert.deepEqual(made, {
      action: { $type: "Answered", questionId: "q1", value: 3, done: true, kind: "a", tags: ["x"] }
    });
  });

  test("names the first field it cannot read", () => {
    assert.deepEqual(actionFrom("Answered", fields, { questionId: "q1", value: "" }), {
      field: "value",
      message: "value needs a value."
    });
    assert.deepEqual(actionFrom("Answered", fields, { questionId: "q1", value: "2.5" }), {
      field: "value",
      message: "value needs a whole number."
    });
    assert.equal(actionFrom("Answered", fields, { questionId: "q1", value: "1", tags: "[" }).field, "tags");
  });
});

describe("source spans", () => {
  test("count lines and columns in scalar values from byte offsets", () => {
    const source = 'let a = "é😀"\nlet b = <Card />';
    const index = new SourceIndex(source);
    const start = new TextEncoder().encode(source).indexOf("<".charCodeAt(0));
    assert.deepEqual(index.span(start, start + 8), {
      startByte: start,
      endByte: start + 8,
      startLine: 2,
      startColumn: 9,
      endLine: 2,
      endColumn: 17
    });
    // The closing quote follows a two-byte and a four-byte character: columns count them as one each.
    assert.equal(index.span(15, 16).startColumn, 12);
  });

  test("answer null for an offset inside a character or past the end", () => {
    const index = new SourceIndex("é");
    assert.equal(index.span(1, 2), null);
    assert.equal(index.span(0, 3), null);
    assert.notEqual(index.span(0, 2), null);
  });
});
