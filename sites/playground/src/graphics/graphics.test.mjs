import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { evaluateFunction } from "@nx-lang/ir-runtime";
import { programFromImages } from "@nx-lang/previewer";
import { prepareDrawingValue, checkGraphicsValue } from "./prepare.ts";
import { glyphCoverage } from "./cmap.ts";
import { pathData, color } from "./paths.ts";
const host = createNxHost(await loadNxModule());
const prefix =
  'import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>>';
const complete = (body) => prefix + body + "</g.Drawing>";
const measure = async (request) => ({
  request,
  run: {
    font: "fixture-test",
    advance: request.text.length * 8,
    ink: {
      rect: { x0: 0, y0: -12, x1: request.text.length * 8, y1: 3 },
      conservative: true,
    },
    ascent: 12,
    descent: 3,
    baseline: 0,
  },
});
async function prepareDrawing(host, source, measure) {
  let artifact;
  try {
    artifact = host.buildProgramArtifact(source, { fileName: "playground.nx" });
    const program = programFromImages(
      artifact.generateNxIr({ debug: true, modules: [] }),
      "playground.nx",
    );
    const origins = { includeCallSites: true };
    const value = evaluateFunction(program, "root", [], { origins });
    return await prepareDrawingValue(artifact, source, value, origins, measure);
  } catch (error) {
    return {
      source,
      scene: null,
      diagnostics: error.diagnostics ?? [{ message: error.message }],
    };
  } finally {
    artifact?.dispose();
  }
}
test.after(() => host.dispose());
test("drawing evaluates IR, supports aliases/helpers, and never calls HIR", async () => {
  const irHost = {
    buildProgramArtifact(source, options) {
      const artifact = host.buildProgramArtifact(source, options);
      artifact.evaluateNx = () => {
        throw new Error("HIR must not run");
      };
      return artifact;
    },
  };
  const result = await prepareDrawing(
    irHost,
    complete(
      '<g.Star cx=50 cy=50 outerRadius=40 innerRadius=20 pointCount=7/><g.Text fontFamily="Fixture" x=50 y=90 textAnchor=middle>hello</g.Text>',
    ),
    measure,
  );
  assert.deepEqual(result.diagnostics, []);
  assert.equal(
    result.scene.root.children[0].identity.module,
    "@nx/graphics/graphics.nx",
  );
  assert.equal(result.scene.root.children[0].identity.declaration, "Polygon");
  assert.equal(result.scene.root.children[0].path.length, 15);
  assert.equal(result.scene.root.children[1].text.x, 30);
});
test("spoofed nodes, malformed paths, invalid helpers and math failures clear the scene", async () => {
  for (const source of [
    'import "@nx/graphics" as g\nexternal component <Rect width:float64 height:float64/>\nlet root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>><Rect width=1 height=1/></g.Drawing>',
    complete('<g.Path data="M0 0 L"/>'),
    complete("<g.Arc cx=0 cy=0 radius=-1/>"),
    'import "@nx/math" as m\nlet root() = {m.sqrt(-1.0)}',
  ]) {
    const result = await prepareDrawing(host, source, measure);
    assert.equal(result.scene, null);
    assert.ok(result.diagnostics.length > 0);
  }
});
test("font failures and non-finite values cannot pass through JSON as a scene", async () => {
  const result = await prepareDrawing(
    host,
    complete('<g.Text fontFamily="missing">hello</g.Text>'),
    async () => {
      throw Error("missing supplied font");
    },
  );
  assert.equal(result.scene, null);
  assert.match(result.diagnostics[0].message, /font/);
  assert.throws(() => checkGraphicsValue({ x: Infinity }), /Non-finite/);
  assert.throws(() => checkGraphicsValue({ x: NaN }), /Non-finite/);
});
test("pinned font coverage supports combining/Greek and rejects missing glyphs", () => {
  const bytes = readFileSync(
    new URL(
      "../../../../crates/nx-graphics-skia/fixtures/fonts/DejaVuSans.ttf",
      import.meta.url,
    ),
  );
  const covers = glyphCoverage(bytes);
  for (const text of ["office", "e\u0301", "Ελλάδα"]) {
    for (const c of text) {
      assert.ok(covers(c.codePointAt(0)), c);
    }
  }
  assert.equal(covers(0x10ffff), false);
  const manifest = JSON.parse(
    readFileSync(
      new URL(
        "../../../../crates/nx-graphics-skia/fixtures/fonts/manifest.json",
        import.meta.url,
      ),
      "utf8",
    ),
  );
  assert.equal(
    createHash("sha256").update(bytes).digest("hex"),
    manifest[0].sha256,
  );
});
test("Canvas adapter accepts only prepared path commands and numerical colors", () => {
  assert.equal(
    pathData([
      { MoveTo: { x: 0, y: 0 } },
      {
        QuadTo: [
          { x: 1, y: 2 },
          { x: 3, y: 4 },
        ],
      },
      {
        CurveTo: [
          { x: 5, y: 6 },
          { x: 7, y: 8 },
          { x: 9, y: 10 },
        ],
      },
      "ClosePath",
    ]),
    "M0 0 Q1 2 3 4 C5 6 7 8 9 10 Z",
  );
  assert.equal(color([1, 0, 0, 0.5]), "rgba(255,0,0,0.5)");
  assert.equal(color([0.123456789, 0.4, 0.8, 0.5]), "rgba(31.481481195,102,204,0.5)");
  assert.equal(color(null), "none");
});
test("helper output retains construction identity and maps every helper back to its authored call", async () => {
  const { authoredSite } = await import("./selection.ts");
  const source = readFileSync(
    new URL("../examples/nx/graphics.nx", import.meta.url),
    "utf8",
  );
  const result = await prepareDrawing(host, source, measure);
  assert.deepEqual(result.diagnostics, []);
  for (const [i, name] of [
    "RegularPolygon",
    "Star",
    "Arrow",
    "Arc",
    "RingSector",
  ].entries()) {
    const node = result.scene.root.children[i];
    const site = authoredSite(node);
    assert.equal(node.identity.module, "@nx/graphics/graphics.nx");
    assert.equal(node.provenance.module, "@nx/graphics/graphics.nx");
    assert.ok(source.slice(site.start, site.end).startsWith(`<${name}\n`));
  }
});
test("one authored element selects every loop rendering, with UTF-8 source spans", async () => {
  const { authoredSite, selectedNodes } = await import("./selection.ts");
  const source =
    "// α😀\n" +
    complete(
      "{for i in 0..3 { <g.Star cx={15.0 + i * 25.0} cy=50 outerRadius=10 innerRadius=5 pointCount=5/> }}",
    );
  const result = await prepareDrawing(host, source, measure);
  assert.deepEqual(result.diagnostics, []);
  const nodes = result.scene.root.children;
  assert.equal(nodes.length, 3);
  const site = authoredSite(nodes[0]);
  assert.ok(
    new TextDecoder()
      .decode(new TextEncoder().encode(source).slice(site.start, site.end))
      .startsWith("<g.Star "),
  );
  assert.deepEqual(
    selectedNodes(nodes, { source, start: site.start, end: site.end }),
    nodes.map((n) => n.id),
  );
});
test("call-site origins are opt-in and preserve values, costs and construction origins", () => {
  const source = complete(
    "<g.Star cx=50 cy=50 outerRadius=40 innerRadius=20 pointCount=5/>",
  );
  const artifact = host.buildProgramArtifact(source, {
    fileName: "playground.nx",
  });
  try {
    const program = programFromImages(
      artifact.generateNxIr({ debug: true, modules: [] }),
      "playground.nx",
    );
    const origins = {},
      calls = { includeCallSites: true },
      a = {},
      b = {};
    const plain = evaluateFunction(program, "root", [], {
      origins,
      usage: a,
      maxOperations: 5000000,
    });
    const expanded = evaluateFunction(program, "root", [], {
      origins: calls,
      usage: b,
      maxOperations: 5000000,
    });
    assert.deepEqual(plain, expanded);
    assert.deepEqual(a, b);
    assert.ok(calls.entries.some((entry) => entry.callSites.length > 0));
    assert.deepEqual(
      calls.entries.map(({ callSites, ...entry }) => entry),
      origins.entries,
    );
    const invalid = { includeCallSites: "yes" };
    assert.throws(
      () => evaluateFunction(program, "root", [], { origins: invalid }),
      /boolean/,
    );
    assert.deepEqual(invalid.entries, []);
  } finally {
    artifact.dispose();
  }
});

test("helper diagnostics use authored call sites", async () => {
  for (const body of [
    "<g.Star cx=50 cy=50 outerRadius=10 innerRadius=50 pointCount=5/>",
    "<g.RegularPolygon cx=50 cy=50 radius=10 pointCount=2/>",
    "<g.Arrow x1=5 y1=5 x2=5 y2=5 shaftWidth=2 headWidth=4 headLength=3/>",
  ]) {
    const source = complete(body);
    const result = await prepareDrawing(host, source, measure);
    assert.equal(result.scene, null);
    assert.equal(result.diagnostics[0].origin, "source");
    const span = result.diagnostics[0].span;
    assert.ok(span);
    assert.equal(
      new TextDecoder().decode(
        new TextEncoder().encode(source).slice(span.startByte, span.endByte),
      ),
      body,
    );
  }
});

test("legacy color strings are rejected before painting", async () => {
  const result = await prepareDrawing(host, complete(
    '<g.Rect width=10 height=10 fill=<g.Paint.solid color="red"/>/>',
  ), measure);
  assert.equal(result.scene, null);
  assert.ok(result.diagnostics.length > 0);
});

test("identity and scene limits are checked before any browser text measurement", async () => {
  let measured = 0;
  const spy = async (request) => {
    measured++;
    return measure(request);
  };
  const spoof =
    'import "@nx/graphics" as g\nexternal component <Text fontFamily:string text:string/>\nlet root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>><Text fontFamily="Other" text="x"/></g.Drawing>';
  const invalid = await prepareDrawing(host, spoof, spy);
  assert.equal(invalid.scene, null);
  assert.match(invalid.diagnostics[0].message, /identity/);
  const many = await prepareDrawing(
    host,
    complete(
      '{for i in 0..2100 { <g.Text fontFamily="Fixture" text={"Text " + i}/> }}',
    ),
    spy,
  );
  assert.equal(many.scene, null);
  assert.match(many.diagnostics[0].message, /limit/);
  assert.equal(measured, 0);
});

test("many sibling translucent elements fit the concurrent layer budget", async () => {
  const result = await prepareDrawing(
    host,
    complete(
      "{for i in 0..100 { <g.Circle cx={i * 1.0} cy=50 r=3 opacity=0.5/> }}",
    ),
    measure,
  );
  assert.deepEqual(result.diagnostics, []);
  assert.equal(result.scene.root.children.length, 100);
});

test("browser font-measurement failures identify the first requesting Text source", async () => {
  const body =
    '<g.Rect width=5 height=5/><g.Text fontFamily="Fixture" text="hello"/><g.Text fontFamily="Fixture" text="hello"/>';
  const source = complete(body);
  let measured = 0;
  const result = await prepareDrawing(host, source, async () => {
    measured++;
    throw new Error("Fixture is missing glyph U+6F22.");
  });
  assert.equal(result.scene, null);
  assert.equal(measured, 1);
  const diagnostic = result.diagnostics[0];
  assert.equal(diagnostic.origin, "source");
  assert.match(diagnostic.message, /element\/children\/1/);
  assert.equal(
    new TextDecoder().decode(
      new TextEncoder()
        .encode(source)
        .slice(diagnostic.span.startByte, diagnostic.span.endByte),
    ),
    '<g.Text fontFamily="Fixture" text="hello"/>',
  );
});

test("large point lists fail before JSON with the owning element and source span", async () => {
  const source = "<Drawing><Polygon points={points}/></Drawing>";
  const start = source.indexOf("<Polygon");
  const points = new Array(99999).fill({ $type: "Point", x: 1, y: 1 });
  const value = {
    $type: "Drawing",
    children: [
      { $type: "Polygon", points },
      { $type: "Polygon", points },
    ],
  };
  const artifact = {
    prepareGraphics() {
      throw Error("point limits must fail before the bridge");
    },
  };
  const origins = {
    entries: [
      {
        path: "/children/0",
        module: "playground.nx",
        start,
        end: source.indexOf("</Drawing>"),
      },
    ],
  };
  const result = await prepareDrawingValue(
    artifact,
    source,
    value,
    origins,
    measure,
  );
  assert.equal(result.scene, null);
  assert.match(
    result.diagnostics[0].message,
    /path command limit exceeded.*element\/children\/0/,
  );
  assert.equal(result.diagnostics[0].span.startByte, start);
});

test("remaining payload budget errors carry the nearest scene node's source", async () => {
  const source = '<Drawing><Text fontFamily="Fixture"/></Drawing>';
  const start = source.indexOf("<Text");
  const value = {
    $type: "Drawing",
    children: [{ $type: "Text", text: "a".repeat(1000001) }],
  };
  const artifact = {
    prepareGraphics() {
      throw Error("payload budget must fail before the bridge");
    },
  };
  const origins = {
    entries: [
      {
        path: "/children/0",
        module: "playground.nx",
        start,
        end: source.indexOf("</Drawing>"),
      },
    ],
  };
  const result = await prepareDrawingValue(
    artifact,
    source,
    value,
    origins,
    measure,
  );
  assert.match(result.diagnostics[0].message, /limits.*element\/children\/0/);
  assert.equal(result.diagnostics[0].span.startByte, start);
});
