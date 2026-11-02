// Real browser acceptance of compact Visual against the actual NX source/edit pipeline.
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
  await viewer.locator(".compact-sheet").first().waitFor({ timeout: 60_000 });
  const key = (name) => viewer.evaluate((element, name) => element.tree.nodes.find((node) => node.role === "element" && node.name === name)?.key, name);
  const row = (key) => viewer.locator(`[data-key="${key}"]`);
  const head = (key) => row(key).locator(":scope > .sheet-head");
  const details = (key) => row(key).locator(":scope > .compact-properties");
  const star = await key("Star"), circle = await key("Circle"), drawing = await key("Drawing");
  assert.ok(star && circle && drawing);
  await head(star).locator(".sheet-kind").click();
  assert.equal(await details(star).isVisible(), true);
  await head(star).getByRole("button", { name: "Pin properties of Star", exact: true }).click();
  await head(circle).locator(".sheet-kind").click();
  assert.equal(await details(star).isVisible(), true);
  assert.equal(await details(circle).isVisible(), true);
  assert.equal(await details(star).locator(".compact-status").textContent(), "Pinned");

  // Keyboard activation, focus retention, and independent disclosure.
  const circleDetails = head(circle).locator(".compact-details");
  await circleDetails.focus();
  await page.keyboard.press("Enter");
  assert.equal(await details(circle).isVisible(), false);
  assert.equal(await circleDetails.evaluate((button) => button === button.getRootNode().activeElement), true);
  await page.keyboard.press("Space");
  assert.equal(await details(circle).isVisible(), true);

  const cx = details(star).locator(`[data-key="${star}.cx"]`);
  await cx.locator(".editable-value").first().click();
  await viewer.locator(".field").fill("invalid");
  await head(star).locator(".compact-details").click();
  assert.equal(await details(star).isVisible(), true);
  assert.equal(await viewer.locator(".field").textContent(), "invalid");
  await page.getByRole("button", { name: "Source", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "Visual", exact: true }).getAttribute("aria-pressed"), "true");
  await viewer.locator(".field").press("Escape");

  // Editing a pinned element retains the other inspected element and uses real source edits.
  await cx.locator(".editable-value").first().click();
  await viewer.locator(".field").fill("75");
  await viewer.locator(".field").press("Enter");
  await page.waitForFunction(() => document.querySelector("nx-viewer").text.includes("cx=75"));
  assert.equal(await details(star).isVisible(), true);
  assert.equal(await details(circle).isVisible(), true);

  // The complete host state survives Source/Visual and undo still concerns source.
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await page.getByRole("button", { name: "Visual", exact: true }).click();
  assert.equal(await head(star).locator(".compact-pin").getAttribute("aria-pressed"), "true");
  await viewer.locator(".body").focus();
  await page.keyboard.press("Control+z");
  await page.waitForFunction(() => !document.querySelector("nx-viewer").text.includes("cx=75"));
  assert.equal(await head(star).locator(".compact-pin").getAttribute("aria-pressed"), "true");

  await head(drawing).locator(".compact-details").click();
  await head(drawing).locator(".fold").click();
  assert.equal(await details(drawing).isVisible(), true);
  assert.equal(await row(star).count(), 0);
  await head(drawing).locator(".fold").press("Enter");
  assert.equal(await details(star).isVisible(), true);

  // A large real document exercises nested composition and header anchoring.
  await page.goto(base + "survey");
  await viewer.locator(".compact-sheet").first().waitFor({ timeout: 60_000 });
  await page.getByRole("button", { name: "Expand outline", exact: true }).click();
  const compositions = await viewer.locator(".compact-sheet").count();
  assert.ok(compositions > 15, `Expected a larger example, got ${compositions}`);
  const choices = viewer.locator(".compact-sheet");
  const first = choices.nth(2).locator(":scope > .sheet-head");
  await first.locator(".sheet-kind").click();
  const nextKey = await choices.nth(3).getAttribute("data-key");
  const next = head(nextKey);
  await next.scrollIntoViewIfNeeded();
  const before = await next.boundingBox();
  await next.locator(".sheet-kind").click();
  const after = await next.boundingBox();
  assert.ok(Math.abs(before.y - after.y) < 3, `Header moved ${after.y - before.y}px`);
  for (const width of [1000, 720]) {
    await page.setViewportSize({ width, height: 950 });
    await head(nextKey).locator(".compact-pin").focus();
    await page.keyboard.press("Space");
    assert.equal(await head(nextKey).locator(".compact-pin").evaluate((button) => getComputedStyle(button).outlineStyle), "solid");
  }
  await page.getByRole("combobox").first().selectOption("graphics");
  await page.waitForFunction(() => {
    const viewer = document.querySelector("nx-viewer");
    return viewer?.text.includes("RegularPolygon") && viewer.detailsState.pinned.length === 0 && viewer.detailsState.opened.length === 0;
  });
  if (process.env.NX_COMPACT_SCREENSHOT) {
    await page.screenshot({ path: process.env.NX_COMPACT_SCREENSHOT, fullPage: true });
  }
  assert.deepEqual(errors, []);
  console.log("PASS: real compact rows, comparison, editing, undo, view restoration, independent folds, keyboard focus, and large-document anchoring.");
} finally {
  await browser?.close();
  try { process.kill(-server.pid, "SIGTERM"); } catch {}
}
