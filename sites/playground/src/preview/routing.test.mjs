import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { createNxSession } from "../worker/session.ts";
import { outputEvaluation } from "../output/evaluation.ts";
import { initialPreview, reducePreview } from "./model.ts";
import { PreviewWorkbench } from "./workbench.ts";
import { programFromImages } from "@nx-lang/previewer";
import { evaluateFunction } from "@nx-lang/ir-runtime";
import { previewDeclarations } from "./metadata.ts";
import { NxHostCrashedError } from "@nx-lang/sdk-wasm";
import { previewerFor } from "./registry.ts";

const module = await loadNxModule();
const session = createNxSession({ module });
after(() => session.dispose());
let id = 0;
const call = (call) => session.answer({ kind: "preview", id: ++id, call });
const drawing =
  'import "@nx/graphics" as g\n<g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>><g.Rect width=12 height=20/></g.Drawing>';

test("the shared IR result supplies raw Output and graphics Preview, including aliased declarations", async () => {
  const state = await call({ op: "start", source: drawing });
  assert.equal(state.renderer, "graphics");
  assert.deepEqual(state.drawing.diagnostics, []);
  assert.equal(state.drawing.scene.root.children[0].geometry.rect.x1, 12);
  assert.match(state.structured.output.value.text, /width=12/);
  assert.ok(state.structured.selectionTargets.length >= 2);
});

test("graphics matching accepts a genuine alias despite a same-name local declaration", async () => {
  const state = await call({
    op: "start",
    source: drawing.replace(
      "<g.Drawing",
      "external component <Drawing value:int/>\n<g.Drawing",
    ),
  });
  assert.equal(state.renderer, "graphics");
  assert.ok(state.drawing.scene);
});

test("unsupported records, primitives, and same-name spoofs are Output-only", async () => {
  for (const source of [
    "let root() = 42",
    'type Question = { label:string }\n<Question label="Which?"/>',
    'import "@nx/graphics" as g\nexternal component <Drawing width:int/>\n<Drawing width=3/>',
  ]) {
    const state = await call({ op: "start", source });
    assert.equal(state.renderer, null);
    assert.equal(state.problem, null);
    assert.ok(state.structured.output.value.text.length > 0);
  }
});

test("an unrelated component declaration does not replace the document's root value", async () => {
  const state = await call({
    op: "start",
    source:
      'external component <Label text:string/>\ncomponent <Unused/> = { <Label text="unused"/> }\nlet root() = 42',
  });
  assert.equal(state.problem, null, JSON.stringify(state.diagnostics));
  assert.equal(state.renderer, null);
  assert.equal(state.structured.output.value.text, "42");
});

test("a custom previewer can register an application type by declaration identity", () => {
  const host = createNxHost(module);
  const artifact = host.buildProgramArtifact(
    'type Question = { label:string }\n<Question label="Which?"/>',
    { fileName: "playground.nx" },
  );
  try {
    const program = programFromImages(
      artifact.generateNxIr({ debug: true, modules: [] }),
      "playground.nx",
    );
    const origins = { includeCallSites: true };
    const value = evaluateFunction(program, "root", [], { origins });
    assert.equal(
      previewerFor(
        program,
        value,
        origins.entries,
        [
          {
            module: "playground.nx",
            name: "Question",
            kind: "record",
            visualizer: { preview: { renderer: "survey" } },
          },
        ],
        [{ id: "survey", renderer: "survey" }],
      ),
      "survey",
    );
    assert.equal(
      previewerFor(
        program,
        value,
        origins.entries,
        [
          {
            module: "another.nx",
            name: "Question",
            kind: "record",
            visualizer: { preview: { renderer: "survey" } },
          },
        ],
        [{ id: "survey", renderer: "survey" }],
      ),
      null,
    );
  } finally {
    artifact.dispose();
    host.dispose();
  }
});

test("renderer failures retain the raw value and preview capability", async () => {
  const state = await call({
    op: "start",
    source: drawing.replace("width=12", "width=-12"),
  });
  assert.equal(state.renderer, "graphics");
  assert.equal(state.drawing.scene, null);
  assert.ok(state.drawing.diagnostics.length > 0);
  assert.match(state.structured.output.value.text, /width=-12/);
});

test("compile and evaluation failures preserve prior availability, value, and source guards", async () => {
  await call({ op: "start", source: drawing });
  for (const broken of [
    drawing + "\nlet broken =",
    drawing.replace("width=12", "width={1 / 0}"),
  ]) {
    const state = await call({ op: "reload", source: broken });
    assert.equal(state.renderer, "graphics");
    assert.equal(state.source, drawing);
    assert.ok(state.diagnostics.length > 0);
    const output = outputEvaluation(
      { ...initialPreview, state, asked: broken },
      broken,
    );
    assert.equal(output.stale, true);
    assert.equal(output.diagnosticsSource, broken);
    assert.ok(state.structured.output);
  }
});

test("shared evaluation does not invoke the legacy interpreter and caches each root", () => {
  const host = createNxHost(module);
  let builds = 0;
  const workbench = new PreviewWorkbench(
    (source) => {
      builds++;
      const artifact = host.buildProgramArtifact(source, {
        fileName: "playground.nx",
      });
      artifact.evaluateNx = () => {
        throw new Error("legacy interpreter must not run");
      };
      return artifact;
    },
    (source) => previewDeclarations(host, source),
  );
  try {
    const first = workbench.answer({ op: "start", source: drawing });
    const next = workbench.answer({ op: "start", source: drawing });
    assert.equal(first.renderer, "graphics");
    assert.deepEqual(first.structured.output, next.structured.output);
    assert.equal(builds, 1);
  } finally {
    workbench.dispose();
    host.dispose();
  }
});

test("an obsolete document answer cannot replace a newer requested source", () => {
  const newer = { op: "start", source: "new" };
  let model = reducePreview(initialPreview, { kind: "asked", call: newer });
  assert.equal(
    reducePreview(model, {
      kind: "answered",
      call: { op: "start", source: "old" },
      state: {},
    }),
    model,
  );
});

test("a same-name runnable component cannot intercept a graphics value from another module", async () => {
  const source = drawing.replace(
    "<g.Drawing",
    "component <Drawing/> = { <g.Circle cx=0 cy=0 r=1/> }\n<g.Drawing",
  );
  const state = await call({ op: "start", source });
  assert.equal(state.problem, null, JSON.stringify(state.diagnostics));
  assert.equal(state.renderer, "graphics");
  assert.ok(state.drawing.scene);
  assert.equal(state.session, null);
});

test("the shared output path retains the playground's source byte limit", async () => {
  const state = await call({
    op: "start",
    source: "//" + "x".repeat(256 * 1024),
  });
  assert.equal(state.problem.kind, "programFailed");
  assert.match(
    state.diagnostics[0].message,
    /source exceeds the playground limit/,
  );
});

test("runtime diagnostics keep the source span from the shared IR build", async () => {
  const source = "let root() = { 1 / 0 }";
  const state = await call({ op: "start", source });
  assert.equal(state.problem.kind, "programFailed");
  assert.equal(state.diagnosticsSource, source);
  assert.ok(state.diagnostics[0].span);
  assert.equal(state.diagnostics[0].origin, "source");
});

const twoComponents =
  'external component <Label text:string/>\ncomponent <A/> = { <Label text="A"/> }\ncomponent <B/> = { state { n:int = {1 / 0} } <Label text="B"/> }\n';

test("RF26 failed root component retains Preview and follows the root when corrected", async () => {
  const source = twoComponents + "<A/>";
  await call({ op: "start", source, reset: true });
  const failed = await call({ op: "reload", source: twoComponents + "<B/>" });
  assert.equal(failed.renderer, "component");
  assert.equal(failed.problem.kind, "startFailed");
  assert.deepEqual(failed.components, ["A", "B"]);
  const recovered = await call({ op: "reload", source });
  assert.equal(recovered.renderer, "component");
  assert.equal(recovered.problem, null);
  assert.match(recovered.session.output.value.text, /text="A"/);
});

test("RF26 picking a failing component keeps the picker capability and can switch back", async () => {
  const source = twoComponents + "<A/>";
  const failed = await call({
    op: "start",
    source,
    component: "B",
    reset: true,
  });
  assert.equal(failed.renderer, "component");
  assert.equal(failed.problem.kind, "startFailed");
  assert.deepEqual(failed.components, ["A", "B"]);
  const recovered = await call({ op: "start", source, component: "A" });
  assert.equal(recovered.problem, null);
  assert.ok(recovered.session);
});

test("RF26 component-only document retains missing-prop guidance and Output says no root", async () => {
  const source =
    'external component <Label text:string/>\ncomponent <Greeting name:string/> = { <Label text={name}/> }\ncomponent <Hello/> = { <Label text="hello"/> }';
  const failed = await call({ op: "start", source, reset: true });
  assert.equal(failed.renderer, "component");
  assert.equal(failed.problem.kind, "startFailed");
  assert.equal(failed.problem.missingProp, true);
  assert.equal(
    outputEvaluation(
      { ...initialPreview, state: failed, asked: source },
      source,
    ).outcome.kind,
    "noRoot",
  );
  const recovered = await call({ op: "start", source, component: "Hello" });
  assert.equal(recovered.problem, null);
  assert.ok(recovered.session);
});

test("RF27 opening a broken document clears the previous document's value, capability, and session", async () => {
  for (const source of [drawing, twoComponents + "<A/>"]) {
    await call({ op: "start", source, reset: true });
    const failed = await call({
      op: "start",
      source: "let root() = { 1 + }",
      reset: true,
    });
    assert.equal(failed.renderer, null);
    assert.equal(failed.structured, null);
    assert.equal(failed.session, null);
    assert.equal(failed.component, null);
    assert.deepEqual(failed.components, []);
    assert.equal(failed.problem.kind, "doesNotCompile");
    const after = await call({
      op: "dispatch",
      handler: "/anything",
      action: { $type: "Tapped" },
    });
    assert.equal(after.session, null);
    assert.equal(after.renderer, null);
  }
});

test("NX metadata, rather than a hardcoded Drawing association, controls preview availability", async () => {
  const source = drawing.replace(
    'import "@nx/graphics" as g',
    'import "@nx/graphics" as g\nimport "@nx/visualizer"\nvisualizer g.Drawing = <ElementVisualizer preview=<PreviewVisualizer renderer="unsupported.renderer"/>/>',
  );
  const state = await call({ op: "start", source, reset: true });
  assert.equal(state.problem, null, JSON.stringify(state.diagnostics));
  assert.equal(state.renderer, null);
  assert.match(state.structured.output.value.text, /<Drawing/);
});

test("unavailable application previewers leave the raw value intact without loading code", async () => {
  const source =
    'import "@nx/visualizer"\ntype Question = { label:string }\nvisualizer Question = <ElementVisualizer preview=<PreviewVisualizer renderer="app.survey"/>/>\n<Question label="Which?"/>';
  const state = await call({ op: "start", source, reset: true });
  assert.equal(state.problem, null, JSON.stringify(state.diagnostics));
  assert.equal(state.renderer, null);
  assert.match(state.structured.output.value.text, /Which\?/);
});

test("an imported helper's result discovers its library preview without an authored Drawing constructor", async () => {
  const host = createNxHost(module);
  const source =
    'import "./helper.nx" as helper\nlet root() = { helper.make() }';
  const helper =
    'import "@nx/graphics" as g\nexport let make() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=10 height=10/>/>';
  const workbench = new PreviewWorkbench(
    (source) =>
      host.buildWorkspaceArtifact({
        entry: "playground.nx",
        modules: [
          { identity: "playground.nx", source },
          { identity: "helper.nx", source: helper },
        ],
      }),
    (source) =>
      previewDeclarations(
        {
          createLanguageSnapshot: (docs) =>
            host.createLanguageSnapshot([
              ...docs,
              {
                uri: "nx://test/helper.nx",
                identity: "helper.nx",
                source: helper,
              },
            ]),
        },
        source,
      ),
  );
  try {
    const state = await workbench.render(
      workbench.answer({ op: "start", source }),
    );
    assert.equal(state.problem, null, JSON.stringify(state.diagnostics));
    assert.equal(state.renderer, "graphics");
    assert.deepEqual(state.drawing.diagnostics, []);
    assert.ok(state.drawing.scene);
  } finally {
    workbench.dispose();
    host.dispose();
  }
});

test("RF32 metadata errors leave Output intact and are cached; host crashes still propagate", () => {
  const host = createNxHost(module);
  let requests = 0;
  const workbench = new PreviewWorkbench(
    (source) =>
      host.buildProgramArtifact(source, { fileName: "playground.nx" }),
    () => {
      requests++;
      throw new Error("metadata unavailable");
    },
  );
  try {
    for (const request of [
      { op: "start", source: drawing },
      { op: "dispatch", handler: "/unused", action: { $type: "Tapped" } },
      { op: "goTo", tick: 0 },
      { op: "start", source: drawing },
    ]) {
      const state = workbench.answer(request);
      assert.equal(state.problem, null);
      assert.equal(state.renderer, null);
      assert.match(state.structured.output.value.text, /<Drawing/);
    }
    assert.equal(requests, 1);
  } finally {
    workbench.dispose();
    host.dispose();
  }
  const crashHost = createNxHost(module);
  const crashed = new PreviewWorkbench(
    (source) =>
      crashHost.buildProgramArtifact(source, { fileName: "playground.nx" }),
    () => {
      throw new NxHostCrashedError("metadata", new Error("trap"));
    },
  );
  try {
    assert.throws(
      () => crashed.answer({ op: "start", source: drawing }),
      NxHostCrashedError,
    );
  } finally {
    crashed.dispose();
    crashHost.dispose();
  }
});

test("RF35 the production metadata lookup runs once per build and always releases its snapshot", () => {
  const host = createNxHost(module);
  let created = 0,
    queried = 0,
    disposed = 0;
  const observed = {
    createLanguageSnapshot: (docs) => {
      created++;
      const snapshot = host.createLanguageSnapshot(docs);
      return {
        sourceTree: (...args) => {
          queried++;
          return snapshot.sourceTree(...args);
        },
        dispose: () => {
          disposed++;
          snapshot.dispose();
        },
      };
    },
  };
  const workbench = new PreviewWorkbench(
    (source) =>
      host.buildProgramArtifact(source, { fileName: "playground.nx" }),
    (source) => previewDeclarations(observed, source),
  );
  try {
    workbench.answer({ op: "start", source: drawing });
    workbench.answer({
      op: "dispatch",
      handler: "/unused",
      action: { $type: "Tapped" },
    });
    workbench.answer({ op: "goTo", tick: 0 });
    workbench.answer({ op: "start", source: drawing });
    assert.deepEqual([created, queried, disposed], [1, 1, 1]);
    workbench.answer({
      op: "reload",
      source: drawing.replace("width=12", "width=13"),
    });
    assert.deepEqual([created, queried, disposed], [2, 2, 2]);
    let released = false;
    assert.throws(
      () =>
        previewDeclarations(
          {
            createLanguageSnapshot: () => ({
              sourceTree: () => {
                throw new Error("query failed");
              },
              dispose: () => {
                released = true;
              },
            }),
          },
          drawing,
        ),
      /query failed/,
    );
    assert.equal(released, true);
  } finally {
    workbench.dispose();
    host.dispose();
  }
});

test("formatting failures retain the evaluated value and registered graphics preview", async () => {
  const host = createNxHost(module);
  const workbench = new PreviewWorkbench(
    (source) => {
      const artifact = host.buildProgramArtifact(source, {
        fileName: "playground.nx",
      });
      artifact.formatValue = () => {
        throw new Error("formatter unavailable");
      };
      return artifact;
    },
    (source) => previewDeclarations(host, source),
  );
  try {
    const state = workbench.answer({ op: "start", source: drawing });
    assert.equal(state.renderer, "graphics");
    assert.equal(state.problem, null);
    assert.match(
      state.structured.output.diagnostics[0].message,
      /formatter unavailable/,
    );
    const rendered = await workbench.render(state);
    assert.ok(rendered.drawing.scene);
  } finally {
    workbench.dispose();
    host.dispose();
  }
});

test("nested graphics reach a bounded graphics diagnostic instead of losing Preview", async () => {
  const source =
    'import "@nx/graphics" as g\n<g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>>' +
    "<g.Group>".repeat(63) +
    "<g.Rect width=1 height=1/>" +
    "</g.Group>".repeat(63) +
    "</g.Drawing>";
  const state = await call({ op: "start", source });
  assert.equal(state.renderer, "graphics");
  assert.equal(state.problem, null);
  assert.equal(state.drawing.scene, null);
  assert.match(state.drawing.diagnostics[0].message, /depth limit/);
  assert.match(state.drawing.diagnostics[0].message, /element\/children\/0/);
  assert.notEqual(state.drawing.diagnostics[0].span, null);
});

test("a fresh replacement workbench honors the picked component on reload", async () => {
  const host = createNxHost(module);
  const workbench = new PreviewWorkbench((source) =>
    host.buildProgramArtifact(source, { fileName: "playground.nx" }),
  );
  try {
    const source =
      'component <A/> = { <Label text="A"/> }\ncomponent <B/> = { <Label text="B"/> }\n<A/>';
    const state = workbench.answer({ op: "reload", source, component: "B" });
    assert.equal(state.component, "B");
    assert.match(state.session.output.value.text, /text="B"/);
  } finally {
    workbench.dispose();
    host.dispose();
  }
});
