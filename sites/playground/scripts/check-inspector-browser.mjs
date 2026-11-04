// Real browser acceptance of Visual rows and inspectors against the actual NX source and edit pipeline.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const root = fileURLToPath(new URL("..", import.meta.url));
const socket = createServer();
await new Promise((resolve) => socket.listen(0, "127.0.0.1", resolve));
const port = socket.address().port;
await new Promise((resolve) => socket.close(resolve));
const server = spawn("pnpm", ["exec", "vite", "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
  cwd: root, detached: true, stdio: ["ignore", "pipe", "pipe"],
});
let log = "";
server.stdout.on("data", (data) => { log += data; });
server.stderr.on("data", (data) => { log += data; });
const base = `http://127.0.0.1:${port}/play/`;
let browser;
try {
  for (let attempt = 0; ; attempt++) {
    try { if ((await fetch(base)).ok) { break; } } catch {}
    if (attempt > 100) { throw new Error(log); }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({ executablePath: process.env.CHROME_PATH ?? "/usr/bin/chromium", headless: true });
  const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(base + "gradients");
  const viewer = page.locator("nx-viewer");
  await viewer.locator(".row-element").first().waitFor({ timeout: 60_000 });
  const key = (name) => viewer.evaluate((element, name) => element.tree.nodes.find((node) => node.role === "element" && node.name === name)?.key, name);
  const row = (key) => viewer.locator(`[data-key="${key}"]`);
  const head = (key) => row(key).locator(":scope > .row-head");
  const inspector = (key) => row(key).locator(":scope > .inspector");
  const star = await key("Star"), circle = await key("Circle"), drawing = await key("Drawing");
  assert.ok(star && circle && drawing);

  // Selecting opens the inspector under the row; clicking the selected row closes and reopens it.
  await head(star).locator(".sheet-kind").click();
  assert.equal(await inspector(star).isVisible(), true);
  await head(star).locator(".sheet-kind").click();
  assert.equal(await inspector(star).count(), 0);
  assert.equal(await viewer.evaluate((element) => element.selection), star);
  await head(star).locator(".sheet-kind").click();
  assert.equal(await inspector(star).isVisible(), true);

  // A pin keeps Star open while Circle is selected; the row the reader acts on stays put.
  await head(star).getByRole("button", { name: "Pin Star", exact: true }).click();
  const before = await head(circle).boundingBox();
  await head(circle).locator(".sheet-kind").click();
  const after = await head(circle).boundingBox();
  assert.ok(Math.abs(before.y - after.y) < 3, `Circle's row moved ${after.y - before.y}px`);
  assert.equal(await inspector(star).isVisible(), true);
  assert.equal(await inspector(circle).isVisible(), true);

  // Scrubbing Star's outer radius goes live and commits once, as one undo step.
  const radius = row(`${star}.outerRadius`);
  const from = Number((await radius.locator(".sheet-value").textContent()).replace(/[^0-9.-]/g, ""));
  const lead = await radius.locator(":scope > .field-lead").boundingBox();
  await page.mouse.move(lead.x + lead.width / 2, lead.y + lead.height / 2);
  await page.mouse.down();
  for (let step = 1; step <= 10; step++) {
    await page.mouse.move(lead.x + lead.width / 2 + 4 + step * 3, lead.y + lead.height / 2);
  }
  await page.mouse.up();
  await page.waitForFunction((from) => {
    const value = Number(document.querySelector("nx-viewer").text.match(/outerRadius=(-?[0-9.]+)/)?.[1]);
    return value > from + 5 && value <= from + 10;
  }, from);
  await viewer.locator(".body").focus();
  await page.keyboard.press("Control+z");
  await page.waitForFunction((text) => document.querySelector("nx-viewer").text.includes(text), `outerRadius=${from}`);

  // An invalid entry keeps its inspector and field open, even across Source and back.
  const cx = row(`${star}.cx`);
  await cx.locator(".editable-value").first().click();
  await viewer.locator(".field").fill("invalid");
  await head(circle).locator(".sheet-kind").click();
  assert.equal(await inspector(star).isVisible(), true);
  assert.equal(await viewer.locator(".field").textContent(), "invalid");
  await viewer.locator(".field").press("Escape");

  // The host keeps pins across Source and Visual.
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  assert.equal(await head(star).locator(".row-pin").getAttribute("aria-pressed"), "true");

  // Folding children leaves the inspector as it is.
  await head(drawing).locator(".sheet-kind").click();
  await head(drawing).locator(".fold").click();
  assert.equal(await inspector(drawing).isVisible(), true);
  assert.equal(await row(circle).count(), 0);
  await head(drawing).locator(".fold").click();

  // Keyboard: the selected row's pin is a Tab stop with a visible focus ring.
  await head(circle).locator(".sheet-kind").click();
  await page.keyboard.press("Tab");
  assert.equal(await head(circle).locator(".row-pin").evaluate((button) => button.getRootNode().activeElement === button), true, "Tab reaches the pin");
  await page.keyboard.press("Space");
  assert.equal(await head(circle).locator(".row-pin").getAttribute("aria-pressed"), "true");
  assert.equal(await head(circle).locator(".row-pin").evaluate((button) => getComputedStyle(button).outlineStyle), "solid");

  // An inspector shows what its element sets; its last line, reached by Tab from the last field,
  // shows the rest and hides them again.
  const more = inspector(circle).locator(":scope > .inspector-more");
  const shown = () => inspector(circle).locator(":scope > .inspector-fields > .prop-field").count();
  const setCount = await shown();
  assert.match(await more.textContent(), /^Show [0-9]+ unset$/);
  await inspector(circle).locator(".editable-value").last().click();
  await page.keyboard.press("Tab");
  assert.equal(await more.evaluate((button) => button.getRootNode().activeElement === button), true, "Tab from the last field reaches the last line");
  await page.keyboard.press("Enter");
  assert.equal(await more.textContent(), "Hide unset");
  assert.ok(await shown() > setCount);
  await more.click();
  assert.equal(await shown(), setCount);

  // A new document drops the pins.
  await page.getByRole("combobox").first().selectOption("graphics");
  await page.waitForFunction(() => {
    const viewer = document.querySelector("nx-viewer");
    return viewer?.text.includes("RegularPolygon") && viewer.detailsState.pinned.length === 0;
  });
  await viewer.locator(".row-element").first().waitFor();
  // A root element's icon starts where the lets before it do.
  const [rootIcon, letStart] = await viewer.evaluate((element) => {
    const root = element.shadowRoot;
    const drawingRow = [...root.querySelectorAll(".row-element")].find((row) => row.querySelector(":scope > .row-head > .sheet-kind")?.textContent === "Drawing");
    const keyword = [...root.querySelectorAll(".t")].find((token) => token.textContent === "let");
    return [drawingRow.querySelector(":scope > .row-head > .element-icon").getBoundingClientRect().left, keyword.getBoundingClientRect().left];
  });
  assert.ok(Math.abs(rootIcon - letStart) < 0.5, `Drawing's icon starts ${rootIcon - letStart}px from let`);
  // Rows, and the if lines among them, keep a steady rhythm: each starts one source line below the last.
  await page.getByRole("combobox").first().selectOption("orders-dashboard");
  await page.waitForFunction(() => document.querySelector("nx-viewer")?.text.includes("CustomerLine"));
  await viewer.locator(".row-element").first().waitFor();
  const tops = await viewer.evaluate((element) => {
    const declaration = [...element.shadowRoot.querySelectorAll(".fold-declaration")].find((node) => node.textContent.startsWith("let <CustomerLine"));
    const top = (node) => node.getBoundingClientRect().top;
    const rows = [...declaration.querySelectorAll(".row-head")].map(top);
    const lines = [...declaration.querySelectorAll(".block-line > .t")].filter((node) => /^(if|})$/.test(node.textContent.trim())).map(top);
    const line = parseFloat(getComputedStyle(declaration).lineHeight);
    return { line, all: [...rows, ...lines].sort((a, b) => a - b) };
  });
  assert.equal(tops.all.length, 6, "Row, two Texts, if, Badge and the closing brace");
  for (let index = 1; index < tops.all.length; index++) {
    assert.ok(Math.abs(tops.all[index] - tops.all[index - 1] - tops.line) < 0.5, `line ${index} is ${tops.all[index] - tops.all[index - 1]}px below the last`);
  }

  if (process.env.NX_INSPECTOR_SCREENSHOT) {
    await page.screenshot({ path: process.env.NX_INSPECTOR_SCREENSHOT, fullPage: true });
  }
  assert.deepEqual(errors, []);
  console.log("PASS: rows, inspectors, close and reopen, pins, anchoring, scrubbing with one undo step, invalid entries, view restoration, folds, keyboard focus, revealing unset properties and row rhythm.");
} finally {
  await browser?.close();
  try { process.kill(-server.pid, "SIGTERM"); } catch {}
}
