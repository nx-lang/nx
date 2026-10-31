// Real Chromium integration test; uses an installed browser without downloading one.
import assert from "node:assert/strict";
import { deflateRawSync } from "node:zlib";
import { createServer } from "node:net";
import { readFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const root = fileURLToPath(new URL("..", import.meta.url));
const server = createServer();
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const vite = spawn(
  "pnpm",
  [
    "exec",
    "vite",
    ...(process.env.NX_PLAYGROUND_PREVIEW === "1" ? ["preview"] : []),
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  {
    cwd: root,
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
  },
);
let log = "";
vite.stdout.on("data", (data) => (log += data));
vite.stderr.on("data", (data) => (log += data));
const base = `http://127.0.0.1:${port}/play/`;
let browser;
let page;
try {
  for (let i = 0; ; i++) {
    try {
      if ((await fetch(base)).ok) break;
    } catch {}
    if (i > 200) throw Error(log);
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH ?? "/usr/bin/chromium",
    headless: true,
    args: ["--no-sandbox"],
  });
  page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  let dialogs = 0;
  page.on("dialog", async (dialog) => {
    dialogs++;
    await dialog.dismiss();
  });
  await page.goto(base + "graphics");
  const canvas = page.locator("canvas.drawing-canvas[data-revision]");
  const gradientCorpus = JSON.parse(readFileSync(new URL("../../../crates/nx-graphics-skia/fixtures/gradients/sampling.json", import.meta.url), "utf8"));
  const gradientFailures = await page.evaluate(async (cases) => {
    const { canvasPaint } = await import("/play/src/graphics/paint.ts");
    const failures = [];
    for (const test of cases) {
      const surface = new OffscreenCanvas(101,101);
      const ctx = surface.getContext("2d", { colorSpace: "srgb" });
      ctx.fillStyle = canvasPaint(ctx, test.paint);
      ctx.fillRect(0,0,101,101);
      for (const sample of test.samples) {
        const rgba = ctx.getImageData(sample.x,sample.y,1,1).data;
        const actual = [rgba[0]*rgba[3]/65025,rgba[1]*rgba[3]/65025,rgba[2]*rgba[3]/65025,rgba[3]/255];
        if (actual.some((x,i) => Math.abs(x-sample.expected[i]) > 2/255)) failures.push({name:test.name,actual,sample});
      }
    }
    const { hitTest } = await import("/play/src/graphics/hit-test.ts");
    const { renderCanvas } = await import("/play/src/graphics/canvas.ts");
    const bounds = {rect:{x0:0,y0:0,x1:100,y1:100},conservative:true};
    const node = {id:"shape",parent:null,order:0,identity:{module:"test",version:"test",declaration:"Rectangle"},
      provenance:null,accessibility:null,transform:[1,0,0,1,0,0],opacity:1,clip:null,clip_evenodd:false,
      path:[{MoveTo:{x:0,y:0}},{LineTo:{x:100,y:0}},{LineTo:{x:100,y:100}},{LineTo:{x:0,y:100}},"ClosePath"],
      evenodd:false,fill:cases[0].paint,stroke:null,text:null,geometry:bounds,painted:bounds,visible:bounds,children:[]};
    const scene={profile:"graphics-core-v4",revision:"test",config:{width:100,height:100,scale:1},root:node};
    if(hitTest(scene,99,50)?.id!=="shape") failures.push("transparent interval selection");
    const group={...node,id:"group",path:null,fill:null,opacity:.5,children:[node],clip:[{MoveTo:{x:0,y:0}},{LineTo:{x:60,y:0}},{LineTo:{x:60,y:100}},{LineTo:{x:0,y:100}},"ClosePath"]};
    const frame=renderCanvas({...scene,root:group}); const ctx=frame.getContext("2d");
    const inside=ctx.getImageData(25,50,1,1).data,outside=ctx.getImageData(75,50,1,1).data;
    if(Math.abs(inside[3]/255-.375)>2/255 || outside[3]!==0) failures.push({compositing:[...inside],outside:[...outside]});
    node.fill={...node.fill,stops:node.fill.stops.map(s=>({...s,color:[s.color[0],s.color[1],s.color[2],0]}))};
    if(hitTest(scene,50,50)!==undefined) failures.push("all-transparent paint hit");
    const { measureBrowserText } = await import("/play/src/graphics/browser-font.ts");
    for(const size of [16,32]) for(const anchor of [0,.5,1]) {
      const measurement=await measureBrowserText({text:"Gradient",family:"Fixture",size,weight:400,italic:false});
      const textNode={...node,id:"text",path:null,transform:[1.25,0,0,.8,10,5],fill:cases[0].paint,
        text:{...measurement,x:55-measurement.run.advance*anchor,y:70}};
      const cfg={...scene.config,width:240,height:110};
      const frame=renderCanvas({...scene,config:cfg,root:textNode});
      const solid=renderCanvas({...scene,config:cfg,root:{...textNode,fill:{kind:"color",color:[1,0,0,1]}}});
      const actual=frame.getContext("2d").getImageData(0,0,240,110).data;
      const reference=solid.getContext("2d").getImageData(0,0,240,110).data;
      let checked=0;
      for(let y=0;y<110;y++) for(let x=0;x<240;x++) {
        const i=(y*240+x)*4;
        if(reference[i+3]<250) continue;
        const localX=(x+.5-10)/1.25;
        const expected=Math.max(0,Math.min(1,1-(localX-.5)/100));
        if(Math.abs(actual[i+3]/255-expected*reference[i+3]/255)>2/255) { if(failures.length<10) failures.push({textSize:size,anchor,x,y,alpha:actual[i+3],expected}); }
        checked++;
      }
      if(!checked) failures.push({emptyText:size,anchor});
    }
    return failures;
  }, gradientCorpus);
  assert.deepEqual(gradientFailures, []);
  const gradientScenes = JSON.parse(readFileSync(new URL("../../../crates/nx-graphics-skia/fixtures/gradients/scenes.json", import.meta.url), "utf8"));
  const sceneFailures = await page.evaluate(async cases => {
    const { renderCanvas } = await import("/play/src/graphics/canvas.ts");
    const failures = [];
    for (const fixture of cases) {
      const ctx = renderCanvas(fixture.prepared).getContext("2d");
      for (const sample of fixture.samples) {
        const rgba = ctx.getImageData(sample.x,sample.y,1,1).data;
        const actual = [rgba[0]*rgba[3]/65025,rgba[1]*rgba[3]/65025,rgba[2]*rgba[3]/65025,rgba[3]/255];
        if (actual.some((v,i) => Math.abs(v-sample.expected[i])>2/255)) failures.push({name:fixture.name,actual,sample});
      }
    }
    return failures;
  },gradientScenes);
  assert.deepEqual(sceneFailures, []);

  console.log("Canvas gradient interpolation matches the shared native analytic corpus.");

  await canvas.waitFor({ timeout: 60_000 });
  assert.equal(
    await page
      .getByRole("button", { name: "Preview", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  assert.ok(
    await page.evaluate(() =>
      [...document.fonts].some(
        (f) => f.family === "NXGraphicsFixtureV1" && f.status === "loaded",
      ),
    ),
  );
  const pixels = await canvas.evaluate((canvas) => {
    const ctx = canvas.getContext("2d");
    return [
      [256, 304],
      [304, 304],
      [112, 304],
    ].map(([x, y]) => [...ctx.getImageData(x, y, 1, 1).data]);
  });
  assert.deepEqual(
    pixels[0],
    pixels[1],
    "group overlap must composite only once",
  );
  assert.deepEqual(
    pixels[2],
    [255, 255, 255, 255],
    "ring center must remain a hole",
  );
  console.log("Initial Canvas frame and opacity/hole pixels passed.");
  // Preview -> Visual preserves the current mode, including expanded library helpers.
  const viewer = page.locator("nx-viewer");
  await viewer.waitFor();
  const graphicsIcons = await viewer.evaluate((viewer) =>
    Object.fromEntries(
      viewer.tree.declarations
        .filter(
          (declaration) => declaration.module === "@nx/graphics/graphics.nx",
        )
        .map((declaration) => [declaration.name, declaration.visualizer?.icon]),
    ),
  );
  for (const [name, icon] of Object.entries({
    Drawing: "frame",
    Group: "group",
    Rectangle: "rectangle-horizontal",
    Circle: "circle",
    Text: "type",
    RegularPolygon: "hexagon",
    Star: "star",
    Arrow: "move-right",
    Arc: "radius",
    RingSector: "donut",
  })) {
    assert.equal(graphicsIcons[name], icon, `Library-owned icon for ${name}`);
  }
  await page.waitForFunction(
    () =>
      document
        .querySelector("nx-viewer")
        ?.shadowRoot.querySelector(".element-icon svg") != null,
  );
  console.log(
    "Library-owned graphics and helper icons reach Visual and render as SVG.",
  );
  async function selectedSource() {
    return await viewer.evaluate((viewer) => {
      const node = viewer.tree?.nodes.find(
        (node) => node.key === viewer.selection,
      );
      return node === undefined
        ? null
        : new TextDecoder().decode(
            new TextEncoder()
              .encode(viewer.text)
              .slice(node.range.startByte, node.range.endByte),
          );
    });
  }
  for (const [id, name] of [
    [0, "RegularPolygon"],
    [1, "Star"],
    [2, "Arrow"],
    [3, "Arc"],
    [4, "RingSector"],
  ]) {
    await page
      .getByLabel("Drawing element")
      .selectOption(`element/children/${id}`);
    assert.equal(
      await page
        .getByRole("button", { name: "Visual", exact: true })
        .getAttribute("aria-pressed"),
      "true",
    );
    assert.ok((await selectedSource())?.startsWith(`<${name}\n`), name);
  }
  await canvas.click({ position: { x: 240, y: 136 } });
  assert.ok((await selectedSource())?.startsWith("<Star\n"));
  async function selectVisual(prefix) {
    await page.waitForFunction(() => {
      const viewer = document.querySelector("nx-viewer");
      return viewer?.tree != null && !viewer.stale;
    });
    const key = await viewer.evaluate((viewer, prefix) => {
      const bytes = new TextEncoder().encode(viewer.text);
      return viewer.tree.nodes.find((node) =>
        new TextDecoder()
          .decode(bytes.slice(node.range.startByte, node.range.endByte))
          .startsWith(prefix),
      )?.key;
    }, prefix);
    assert.ok(key, `Visual node ${prefix} exists`);
    // Drive the same selection event the Visual view emits, avoiding editable-property clicks.
    await viewer.evaluate((viewer, key) => {
      viewer.selection = key;
      viewer.dispatchEvent(
        new CustomEvent("nx-select", {
          detail: { key },
          bubbles: true,
          composed: true,
        }),
      );
    }, key);
    return key;
  }
  await selectVisual("<Arrow\n");
  await page.waitForFunction(() =>
    JSON.parse(
      document.querySelector("canvas.drawing-canvas").dataset.selectedIds,
    ).includes("element/children/2"),
  );
  const before = await canvas.getAttribute("data-revision");
  await page.getByRole("button", { name: "Source", exact: true }).click();
  const editor = page.locator(".monaco-editor").first();
  await editor.waitFor();
  async function edit(source) {
    await editor.click({ position: { x: 100, y: 40 } });
    await page.keyboard.press("Control+A");
    await page.keyboard.insertText(source);
  }
  const original = readFileSync(
    new URL("../src/examples/nx/graphics.nx", import.meta.url),
    "utf8",
  );
  // Color tooling works in an application that imports no graphics library.
  const colorOnlySource = 'import "@nx/color" as c\nlet accent = #33669980\nlet calculated = { c.withAlpha(accent, 0.5) }\nlet root() = { accent }';
  await edit(colorOnlySource);
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  await page.waitForFunction(() => {
    const v = document.querySelector("nx-viewer");
    return v?.text.includes("let accent = #33669980") && !v.stale;
  });
  const literalKey = await viewer.evaluate(v => v.tree.nodes.find(n => n.value === "#33669980")?.key);
  assert.ok(literalKey);
  const colorNode = viewer.locator(`[data-key=${JSON.stringify(literalKey)}]`).last();
  await colorNode.click();
  const colorPicker = viewer.locator(".popover-picker");
  await colorPicker.waitFor({ state: "visible" });
  assert.equal(await viewer.locator(".popover-alpha").getAttribute("aria-label") !== null, true);
  await colorPicker.evaluate(picker => {
    picker.value = "#aabbcc";
    picker.dispatchEvent(new Event("input", { bubbles: true }));
    picker.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await page.waitForFunction(() => document.querySelector("nx-viewer")?.text.includes("#aabbcc80"));
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await editor.click({ position: { x: 100, y: 40 } });
  await page.keyboard.press("Control+z");
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  await page.waitForFunction(() => document.querySelector("nx-viewer")?.text.includes("#33669980"));
  const computedEditable = await viewer.evaluate(v => {
    const call = v.tree.nodes.find(n => n.role === "call" && v.text.slice(n.range.startByte,n.range.endByte).startsWith("c.withAlpha"));
    return call?.color !== undefined;
  });
  assert.equal(computedEditable, false, "computed colors stay read-only");
  assert.equal(await page.locator("canvas.drawing-canvas[data-revision]").count(), 0, "color-only source activates no graphics preview");
  console.log("Color-only literal picker preserves alpha, supports undo, and leaves computed colors read-only.");
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await edit(original);
  await canvas.waitFor({ timeout: 60_000 });
  await edit(original.replace("pointCount=5", "pointCount=7"));
  await page.waitForFunction((old) => {
    const canvas = document.querySelector(
      "canvas.drawing-canvas[data-revision]",
    );
    return canvas !== null && canvas.dataset.revision !== old;
  }, before);
  console.log("Source edit produced a new drawing revision.");
  assert.equal(
    await page.getByRole("button", { name: "Drawing", exact: true }).count(),
    0,
  );
  await page.getByRole("button", { name: "Output", exact: true }).click();
  const rawGraphics = page.locator(
    ".pane-output .output-body:not([hidden]) nx-value",
  );
  await rawGraphics.waitFor();
  assert.match(
    await rawGraphics.evaluate((element) => element.value.text),
    /<Drawing/,
  );
  const outputRevision = await canvas.getAttribute("data-revision");
  await edit(original.replace("pointCount=5", "pointCount=8"));
  await page.waitForFunction(
    (old) =>
      document.querySelector("canvas.drawing-canvas")?.dataset.revision !== old,
    outputRevision,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Output", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await page.getByRole("button", { name: "Preview", exact: true }).click();
  console.log(
    "Graphics raw Output shares the current result and preserves its tab across edits.",
  );
  await page.getByLabel("Drawing element").selectOption("element/children/1");
  await page.getByText("Bounds and text metrics", { exact: true }).click();
  assert.match(
    await page.locator(".drawing-info pre").textContent(),
    /Polygon/,
  );
  // Source-mode cursor movement also drives selection, clearing the previous output click.
  await editor.click({ position: { x: 100, y: 40 } });
  await page.keyboard.press("Control+Home");
  const arrowLine =
    original.slice(0, original.indexOf("<Arrow")).split("\n").length - 1;
  for (let i = 0; i < arrowLine; i++) {
    await page.keyboard.press("ArrowDown");
  }
  await page.keyboard.press("Home");
  for (let i = 0; i < 4; i++) {
    await page.keyboard.press("ArrowRight");
  }
  await page.waitForFunction(() =>
    JSON.parse(
      document.querySelector("canvas.drawing-canvas").dataset.selectedIds,
    ).includes("element/children/2"),
  );
  const prefix =
    'import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.Rect x=0 y=0 width=100 height=100/>>';
  async function draw(body) {
    const old = (await canvas.count())
      ? await canvas.getAttribute("data-revision")
      : null;
    await edit(prefix + body + "</g.Drawing>");
    await page.waitForFunction((old) => {
      const canvas = document.querySelector(
        "canvas.drawing-canvas[data-revision]",
      );
      return canvas !== null && canvas.dataset.revision !== old;
    }, old);
  }
  const gradientStops = '<g.GradientStop offset=0 color=#ff0000/><g.GradientStop offset=1 color=#0000ff00/>';
  const linear = `<g.Paint.linearGradient x1=0 y1=0 x2=100 y2=0 stops={${gradientStops}}/>`;
  const radial = `<g.Paint.radialGradient cx=50 cy=50 radius=45 stops={${gradientStops}}/>`;
  await draw(`<g.Rectangle width=100 height=100 fill=${linear}/><g.Circle cx=50 cy=50 r=30 fill=${radial}/><g.Line x1=10 y1=90 x2=90 y2=90 stroke=<g.Stroke paint=${linear} width=3/>/><g.Text x=10 y=20 fontFamily="Fixture" fill=${linear}>Gradient</g.Text>`);
  const gradientPixels = await canvas.evaluate(c => {
    const ctx=c.getContext("2d"); const y=Math.round(c.height*.8);
    return [.2,.8].map(f => [...ctx.getImageData(Math.round(c.width*f),y,1,1).data]);
  });
  assert.ok(gradientPixels[0][0] > 245 && gradientPixels[0][3] > gradientPixels[1][3]);
  await edit(prefix + `<g.Rectangle width=100 height=100 fill=<g.Paint.linearGradient x1=0 y1=0 x2=0 y2=0 stops={${gradientStops}}/>/></g.Drawing>`);
  await page.getByRole("alert").filter({hasText:"Drawing could not be rendered"}).waitFor();
  assert.equal(await canvas.count(),0);
  await edit(prefix + '<g.Path data="M0 0 L"/></g.Drawing>');
  await page
    .getByRole("alert")
    .filter({ hasText: "Drawing could not be rendered" })
    .waitFor();
  assert.equal(await canvas.count(), 0);
  await edit(prefix + '<g.Text fontFamily="Fixture">漢</g.Text></g.Drawing>');
  await page.getByRole("alert").filter({ hasText: "missing glyph" }).waitFor();
  assert.equal(await canvas.count(), 0);
  console.log("Invalid path and missing glyph cleared the scene.");
  await draw('<g.Text fontFamily="Fixture" text="<script>alert(1)</script>"/>');
  assert.equal(await page.locator(".drawing-view script").count(), 0);
  assert.equal(dialogs, 0);
  // Clip holes use their own rule, independently of the shape's fill rule and transforms.
  await draw(
    '<g.Rectangle width=100 height=100 fill=<g.Paint.color color=#ff0000/> clipPath="M0 0H100V100H0Z M25 25H75V75H25Z" clipRule=evenodd/>',
  );
  const clip = await canvas.evaluate((canvas) => {
    const ctx = canvas.getContext("2d");
    return [
      [40, 40],
      [320, 240],
    ].map(([x, y]) => [...ctx.getImageData(x, y, 1, 1).data]);
  });
  // The square viewBox is centered in the wide viewport (80px left gutter).
  assert.deepEqual(clip[0], [0, 0, 0, 0]);
  assert.deepEqual(clip[1], [0, 0, 0, 0]);
  const solid = await canvas.evaluate((c) => [
    ...c.getContext("2d").getImageData(104, 24, 1, 1).data,
  ]);
  assert.deepEqual(solid, [255, 0, 0, 255]);
  await draw(
    "{for i in 0..3 { <g.Star cx={15.0 + i * 25.0} cy=50 outerRadius=10 innerRadius=5 pointCount=5/> }}",
  );
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  await viewer.waitFor();
  await selectVisual("<g.Star ");
  await page.waitForFunction(
    () =>
      JSON.parse(
        document.querySelector("canvas.drawing-canvas").dataset.selectedIds,
      ).length === 3,
  );
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await edit(original);
  await canvas.waitFor();
  const screenshot = process.env.NX_GRAPHICS_SCREENSHOT;
  await editor.click({ position: { x: 100, y: 40 } });
  await page.keyboard.press("Control+Home");
  if (screenshot) {
    mkdirSync(dirname(screenshot), { recursive: true });
    await page.screenshot({ path: screenshot });
  }
  // A static structured root uses the same contract; no graphics-specific types participate.
  const survey =
    'type Choice = { label:string }\ntype Question = { label:string choices:Choice+ }\nlet root() = <Question label="Which?" choices={<Choice label="A"/><Choice label="B"/>}/>';
  await edit(survey);
  const structured = page
    .locator(".pane-output .output-body:not([hidden]) nx-value")
    .first();
  await page.waitForFunction(() =>
    document
      .querySelector(".pane-output .output-body:not([hidden]) nx-value")
      ?.value?.text.includes("Which?"),
  );
  assert.equal(
    await page.getByRole("button", { name: "Preview", exact: true }).count(),
    0,
  );
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  await viewer.waitFor();
  await selectVisual("<Question ");
  await page.waitForFunction(
    () =>
      document
        .querySelector(".pane-output .output-body:not([hidden]) nx-value")
        ?.shadowRoot.querySelectorAll(".node.selected").length === 3,
  );
  const choice = await structured.evaluate((element) =>
    element.value.nodes.findIndex(
      (node) => node.role === "record" && node.pointer === "/choices/1",
    ),
  );
  assert.ok(choice >= 0);
  const choiceNode = structured.locator(`.node[data-node="${choice}"]`);
  await choiceNode.focus();
  await choiceNode.press("Enter");
  assert.equal(
    await page
      .getByRole("button", { name: "Visual", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await page.waitForFunction(() => {
    const viewer = document.querySelector("nx-viewer");
    const node = viewer?.tree?.nodes.find(
      (node) => node.key === viewer.selection,
    );
    return (
      node != null &&
      new TextDecoder()
        .decode(
          new TextEncoder()
            .encode(viewer.text)
            .slice(node.range.startByte, node.range.endByte),
        )
        .startsWith('<Choice label="B"')
    );
  });
  console.log(
    "Bidirectional selection passed for helpers, repeated loop output, and structured Question/Choice records.",
  );
  await page.locator(".examples select").selectOption("survey");
  await page.waitForFunction(() =>
    document
      .querySelector(".pane-output .output-body:not([hidden]) nx-value")
      ?.value?.nodes.some((node) => node.type === "SelectQuestion"),
  );
  assert.equal(
    await page.getByRole("button", { name: "Preview", exact: true }).count(),
    0,
  );
  await page.locator(".examples select").selectOption("counter");
  await page.locator(".preview .preview-picker select").waitFor();
  assert.equal(
    await page
      .getByRole("button", { name: "Preview", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  const componentValue = page.locator(".preview nx-value").first();
  const handler = await componentValue.evaluate((element) =>
    element.value.nodes.findIndex(
      (node) =>
        node.type === "ActionHandler" && node.pointer === "/content/1/onTapped",
    ),
  );
  assert.ok(handler >= 0);
  await componentValue.evaluate((element, index) => {
    element.dispatchEvent(
      new CustomEvent("nx-value-action", {
        detail: { node: element.value.nodes[index], id: "Button.Tapped" },
        bubbles: true,
        composed: true,
      }),
    );
  }, handler);
  await page.waitForFunction(() =>
    document
      .querySelector(".preview nx-value")
      ?.value?.text.includes("Count: 4"),
  );
  await page.getByRole("button", { name: "Output", exact: true }).click();
  assert.match(
    await page
      .locator(".pane-output .output-body:not([hidden]) nx-value")
      .evaluate((element) => element.value.text),
    /Count: 4/,
  );
  await page.getByRole("button", { name: "Source", exact: true }).click();
  const counterSource = readFileSync(
    new URL("../src/examples/nx/counter.nx", import.meta.url),
    "utf8",
  );
  await edit(
    'import "@nx/graphics" as g\n' +
      counterSource +
      "\nlet deep() = " +
      "<g.Group>".repeat(1000) +
      "</g.Group>".repeat(1000),
  );
  await page
    .locator(".output-body:not([hidden])")
    .getByRole("button", { name: "Start again", exact: true })
    .waitFor({ timeout: 60_000 });
  assert.equal(
    await page.getByRole("button", { name: "Preview", exact: true }).count(),
    1,
  );
  assert.match(
    await page
      .locator(".output-body:not([hidden]) nx-value")
      .evaluate((e) => e.value.text),
    /Count: 4/,
  );
  await edit(counterSource);
  await page.waitForFunction(() => {
    const output = document.querySelector(
      ".output-body:not([hidden]) nx-value",
    );
    return (
      output?.value?.text.includes("Count: 4") &&
      !output.classList.contains("stale") &&
      ![...document.querySelectorAll(".output-body:not([hidden]) button")].some(
        (b) => b.textContent === "Start again",
      )
    );
  });
  console.log(
    "Worker failure preserves stale Output, Preview availability and the counter session on recovery.",
  );
  await page.getByRole("button", { name: "Preview", exact: true }).click();
  await page.getByRole("button", { name: "Back a step" }).click();
  await page.waitForFunction(
    () => document.querySelectorAll(".timeline-steps > li").length === 2,
  );
  await page.getByRole("button", { name: "Back a step" }).click();
  await page.waitForFunction(
    () => document.querySelectorAll(".timeline-steps > li").length === 1,
  );
  await page.getByRole("button", { name: "Output", exact: true }).click();
  assert.match(
    await page
      .locator(".pane-output .output-body:not([hidden]) nx-value")
      .evaluate((element) => element.value.text),
    /Count: 3/,
  );
  console.log(
    "Component actions and timeline update the same value shown by Output.",
  );
  await page.goto(base + "counter");
  await page.locator(".preview .preview-picker select").waitFor();
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await page.evaluate(() => {
    globalThis.staleFrames = [];
    globalThis.staleObserver = new MutationObserver(() => {
      if (
        [...document.querySelectorAll("nx-value")].some((e) => e.stale) ||
        document.body.textContent.includes(
          "runs the last program that compiled",
        )
      ) {
        globalThis.staleFrames.push({
          values: [...document.querySelectorAll(".pane-output nx-value")]
            .filter((e) => e.stale)
            .map((e) => e.className),
          notice: document.body.textContent.includes(
            "runs the last program that compiled",
          ),
        });
      }
    });
    globalThis.staleObserver.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
    });
  });
  await edit(counterSource + "\n// valid edit regression\n");
  await page.waitForFunction(
    () =>
      document.querySelector(".pane-output .pane-note")?.textContent ===
      "updating…",
  );
  await page.waitForFunction(
    () =>
      document.querySelector(".pane-output .pane-note")?.textContent !==
      "updating…",
  );
  assert.deepEqual(
    await page.evaluate(() => {
      globalThis.staleObserver.disconnect();
      return globalThis.staleFrames;
    }),
    [],
  );
  await edit(counterSource + "\nlet broken =");
  await page.waitForFunction(() =>
    [...document.querySelectorAll(".pane-output nx-value")].every(
      (e) => e.stale,
    ),
  );
  await page.evaluate(() => {
    globalThis.staleFrames = [];
    globalThis.staleObserver = new MutationObserver(() => {
      if (
        [...document.querySelectorAll(".pane-output nx-value")].some(
          (e) => !e.stale,
        )
      )
        globalThis.staleFrames.push(true);
    });
    globalThis.staleObserver.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
    });
  });
  await edit(counterSource + "\nlet broken = ");
  await page.waitForFunction(
    () =>
      document.querySelector(".pane-output .pane-note")?.textContent ===
      "updating…",
  );
  await page.waitForFunction(
    () =>
      document.querySelector(".pane-output .pane-note")?.textContent !==
      "updating…",
  );
  assert.deepEqual(
    await page.evaluate(() => {
      globalThis.staleObserver.disconnect();
      return globalThis.staleFrames;
    }),
    [],
  );
  const deepDocument =
    prefix +
    "<g.Group>".repeat(1000) +
    "</g.Group>".repeat(1000) +
    "</g.Drawing>";
  const failingLink =
    base + "#code=" + deflateRawSync(deepDocument).toString("base64url");
  await page.evaluate((url) => {
    history.pushState(null, "", url);
    dispatchEvent(new PopStateEvent("popstate"));
  }, failingLink);
  await page
    .locator(".output-body:not([hidden])")
    .getByRole("button", { name: "Start again", exact: true })
    .waitFor({ timeout: 60_000 });
  assert.equal(
    await page.getByRole("button", { name: "Preview", exact: true }).count(),
    0,
  );
  assert.equal(await page.locator(".pane-output nx-value").count(), 0);
  console.log(
    "Valid edits do not flash stale, and a failing newly opened document clears the old session.",
  );

  await page.goto(base + "counter");
  await page.locator(".preview .preview-picker select").waitFor();
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await edit(
    "component <Deep/> = { " +
      "<Row>".repeat(300) +
      '<Label text="deep"/>' +
      "</Row>".repeat(300) +
      " }\n<Deep/>",
  );
  await page
    .locator(".preview .output-heading")
    .filter({ hasText: "The value could not be displayed" })
    .waitFor({ timeout: 60_000 });
  await page.getByRole("button", { name: "Output", exact: true }).click();
  assert.equal(
    await page
      .locator(".output-body:not([hidden]) .output-heading")
      .textContent(),
    "The value could not be displayed",
  );
  console.log(
    "Broken edits remain stale while pending, and component formatting headings agree across tabs.",
  );

  await page.goto(base + "graphics");
  await canvas.waitFor({ timeout: 60_000 });
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await draw(
    '<g.Rectangle x=70 y=5 width=20 height=20 fill=<g.Paint.color color=#0000ff/>/><g.Line x1=0 y1=100 x2=100 y2=0 stroke=<g.Stroke paint=<g.Paint.color color=#000000/> width=2/>/>',
  );
  await canvas.evaluate((canvas) => {
    const box = canvas.getBoundingClientRect();
    canvas.dispatchEvent(
      new MouseEvent("click", {
        bubbles: true,
        clientX: box.left + (box.width * 464) / 640,
        clientY: box.top + (box.height * 72) / 480,
      }),
    );
  });
  assert.equal(
    await page.getByLabel("Drawing element").inputValue(),
    "element/children/0",
  );
  console.log(
    "Canvas hit-testing selects the painted Rectangle beneath a Line's bounding box.",
  );

  const textSource =
    prefix +
    '<g.Text fontFamily="Fixture" x=10 y=50 fontSize=4>Hello world</g.Text></g.Drawing>';
  for (const deviceScaleFactor of [1, 2, 4]) {
    const target = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
      deviceScaleFactor,
    });
    try {
      await target.goto(base + "graphics");
      const sharp = target.locator("canvas.drawing-canvas[data-revision]");
      await sharp.waitFor({ timeout: 60_000 });
      const dimensions = await sharp.evaluate((canvas) => ({
        width: canvas.width,
        height: canvas.height,
      }));
      if (deviceScaleFactor === 2)
        assert.deepEqual(dimensions, { width: 1280, height: 960 });
      assert.ok(dimensions.width * dimensions.height <= 4194304);
      await target.getByRole("button", { name: "Source", exact: true }).click();
      async function setSource(source) {
        const before = await sharp.getAttribute("data-revision");
        await target
          .locator(".monaco-editor")
          .first()
          .click({ position: { x: 100, y: 40 } });
        await target.keyboard.press("Control+A");
        await target.keyboard.insertText(source);
        await target.waitForFunction((revision) => {
          const canvas = document.querySelector(
            "canvas.drawing-canvas[data-revision]",
          );
          return canvas != null && canvas.dataset.revision !== revision;
        }, before);
      }
      await setSource(textSource);
      await target
        .getByLabel("Drawing element")
        .selectOption("element/children/0");
      const escaped = await target.evaluate(() => {
        const canvas = document.querySelector("canvas.drawing-canvas");
        const rect = JSON.parse(
          document.querySelector(".drawing-info pre").textContent,
        ).visible.rect;
        const scale = canvas.width / 640;
        const pixels = canvas
          .getContext("2d")
          .getImageData(0, 0, canvas.width, canvas.height).data;
        let escaped = 0;
        for (let y = 0; y < canvas.height; y++)
          for (let x = 0; x < canvas.width; x++) {
            const i = (y * canvas.width + x) * 4;
            if (
              pixels[i + 3] > 0 &&
              pixels[i] < 20 &&
              pixels[i + 1] < 20 &&
              pixels[i + 2] < 20 &&
              (x + 1 <= rect.x0 * scale ||
                x >= rect.x1 * scale ||
                y + 1 <= rect.y0 * scale ||
                y >= rect.y1 * scale)
            )
              escaped++;
          }
        return escaped;
      });
      assert.equal(
        escaped,
        0,
        `text outside conservative bounds at DPR ${deviceScaleFactor}`,
      );
      if (deviceScaleFactor === 4) {
        await setSource(
          prefix +
            "<g.Group opacity=0.9>".repeat(14) +
            "<g.Rectangle width=10 height=10/>" +
            "</g.Group>".repeat(14) +
            "</g.Drawing>",
        );
        assert.ok(await sharp.isVisible());
      }
    } finally {
      await target.close();
    }
  }
  console.log(
    "DPR 1/2/4 text stays inside physical painted bounds; high-DPR surfaces and nested opacity fit resource limits.",
  );

  if (process.env.NX_PLAYGROUND_PREVIEW !== "1") {
    const metrics = await page.evaluate(async () => {
      const { measureBrowserText } = await import(
        "/play/src/graphics/browser-font.ts"
      );
      const request = {
        text: "Hello world",
        family: "Fixture",
        weight: 400,
        italic: false,
      };
      return await Promise.all(
        [0.01, 0.04, 4, 16, 20000].map(async (size) => ({
          size,
          run: (await measureBrowserText({ ...request, size })).run,
        })),
      );
    });
    const em = metrics[0].run.advance / metrics[0].size;
    for (const { size, run } of metrics) {
      assert.ok(Math.abs(run.advance / size - em) < 1e-8);
      assert.ok(run.ink.rect.y0 / size < -0.76);
      assert.ok(run.ink.rect.y1 > 0);
    }
    console.log(
      "Browser text scales consistently from tiny through very large local sizes.",
    );
  }
  await page.goto(base + "gradients");
  await page.locator("canvas.drawing-canvas[data-revision]").waitFor({ state: "visible" });
  const gradientElements = await page.getByRole("combobox", { name: "Drawing element", exact: true })
    .locator("option").allTextContents();
  const gradientDeclarations = gradientElements.map((label) => label.split(" · ")[0].trim());
  assert.ok(gradientDeclarations.includes("Rectangle"));
  assert.ok(!gradientDeclarations.includes("Rect"));
  console.log("The gradients playground example renders with shared Rect data and Rectangle elements.");
  assert.deepEqual(errors, []);
  console.log(
    "Canvas Drawing renders, revises, measures supplied fonts, isolates opacity, respects clip holes, inspects, rejects invalid scenes and preserves component Preview.",
  );
} catch (error) {
  if (page) {
    await page.screenshot({ path: "/tmp/nx-selection-browser-failure.png" });
    console.error(
      await page.locator(".drawing-info, .preview").allTextContents(),
    );
    console.error(
      await page.evaluate(() => {
        const v = document.querySelector("nx-viewer");
        const n = v?.tree?.nodes.find((n) => n.key === v.selection);
        return {
          key: v?.selection,
          node: n,
          ids: document.querySelector("canvas.drawing-canvas")?.dataset
            .selectedIds,
        };
      }),
    );
  }
  throw error;
} finally {
  await browser?.close();
  try {
    process.kill(-vite.pid, "SIGTERM");
  } catch {
    vite.kill();
  }
}
