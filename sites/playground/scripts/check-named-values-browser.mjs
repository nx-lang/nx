// Real browser acceptance of named values in Visual: the popover, choosing a token and undoing it,
// against the actual NX source and edit pipeline. NX_NAMED_SCREENSHOTS names a folder for screenshots
// of the popover in light and dark.
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
  const page = await browser.newPage({ viewport: { width: 1500, height: 1000 }, deviceScaleFactor: process.env.NX_NAMED_SCREENSHOTS ? 2 : 1 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(base + "visual-test-cases");
  const viewer = page.locator("nx-viewer");
  await viewer.locator(".row-element").first().waitFor({ timeout: 60_000 });
  const tile = await viewer.evaluate((element) => element.tree.nodes.find((node) => node.role === "element" && node.name === "Tile" && node.key.startsWith("tiles"))?.key);
  assert.ok(tile, "the tiles example is drawn");
  const row = (key) => viewer.locator(`[data-key="${key}"]`);
  await row(tile).locator(":scope > .row-head .sheet-kind").click();
  const padding = row(`${tile}.padding`);
  await padding.waitFor();
  const token = padding.locator(".reference-token");
  assert.equal((await token.textContent()).replace(/[\s\u22ef]+/g, ""), "{env.space.md}");

  // The hexagon past the value opens the popover, the current name marked.
  await padding.hover();
  await padding.locator(".named-values").click();
  const popover = viewer.locator(".popover.named");
  await popover.waitFor();
  assert.equal(await popover.locator(".named-row.current").getAttribute("data-name"), "env.space.md");
  // It opens on the current value: active, and inside the list's visible part, below its heading.
  assert.equal(await popover.locator(".named-row.active").getAttribute("data-name"), "env.space.md");
  const placed = await popover.evaluate((element) => {
    const list = element.querySelector(".named-list").getBoundingClientRect();
    const current = element.querySelector(".named-row.current").getBoundingClientRect();
    return { list: [list.top, list.bottom], current: [current.top, current.bottom] };
  });
  assert.ok(placed.current[0] >= placed.list[0] + 16 && placed.current[1] <= placed.list[1], JSON.stringify(placed));
  assert.equal(await popover.locator(".named-group").first().textContent(), "Environment");
  const shoot = async (name) => {
    if (process.env.NX_NAMED_SCREENSHOTS) {
      await page.screenshot({ path: `${process.env.NX_NAMED_SCREENSHOTS}/${name}.png`, clip: await popoverClip() });
    }
  };
  const popoverClip = async () => {
    const box = await popover.boundingBox();
    const at = await padding.boundingBox();
    const left = Math.max(0, Math.min(box.x, at.x) - 24);
    const top = Math.max(0, at.y - 40);
    return { x: left, y: top, width: Math.max(box.x + box.width, at.x + at.width) - left + 24, height: box.y + box.height - top + 16 };
  };
  await shoot("named-values-light");

  // Typing filters; Enter chooses; Source gets the name.
  await page.keyboard.type("space lg");
  assert.deepEqual(await popover.locator(".named-row:not([hidden])").evaluateAll((rows) => rows.map((row) => row.dataset.name)), ["env.space.lg"]);
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector("nx-viewer").text.includes("padding={env.space.lg}"));
  await row(`${tile}.padding`).locator(".reference-token").waitFor();
  assert.equal((await row(`${tile}.padding`).locator(".reference-token").textContent()).replace(/[\s\u22ef]+/g, ""), "{env.space.lg}");

  // Undo takes it back.
  await viewer.locator(".body").focus();
  await page.keyboard.press("Control+z");
  await page.waitForFunction(() => document.querySelector("nx-viewer").text.includes("padding={env.space.md}"));

  // A property written as a plain number binds to a token.
  const border = row(`${tile}.border`);
  await border.hover();
  await border.locator(".named-values").click();
  await popover.waitFor();
  assert.equal(await popover.locator(".named-row.current").count(), 0, "a literal marks no row");
  await popover.locator('.named-row[data-name="env.borderWidth.thin"]').click();
  await page.waitForFunction(() => document.querySelector("nx-viewer").text.includes("border={env.borderWidth.thin}"));
  await viewer.locator(".body").focus();
  await page.keyboard.press("Control+z");
  await page.waitForFunction(() => document.querySelector("nx-viewer").text.includes("border=1"));

  // An unset property's hexagon adds it as a token.
  const unset = viewer.locator(`[data-named-key="${tile}.radius"].sheet-row`);
  if ((await unset.count()) === 0) {
    await row(tile).locator(".inspector-more").click();
  }
  await unset.hover();
  await unset.locator(".named-values").click();
  await popover.locator('.named-row[data-name="env.radius.lg"]').click();
  await page.waitForFunction(() => /radius=\{env\.radius\.lg\}/.test(document.querySelector("nx-viewer").text));
  await viewer.locator(".body").focus();
  await page.keyboard.press("Control+z");
  await page.waitForFunction(() => !document.querySelector("nx-viewer").text.includes("env.radius.lg"));

  // In dark, a color's popover shows its swatches.
  await page.emulateMedia({ colorScheme: "dark" });
  const fill = row(`${tile}.fill`);
  await fill.locator(".reference-token").click();
  await popover.waitFor();
  assert.ok((await popover.locator(".named-row .swatch").count()) > 5);
  // Palette rows read their whole name, without the import alias, and none is cut.
  const palette = await popover.evaluate((element) => {
    const names = [...element.querySelectorAll(".named-row")].filter((row) => /standardPalette\./.test(row.dataset.name ?? "")).map((row) => row.querySelector(".named-name"));
    names[0]?.scrollIntoView({ block: "center" });
    return { rows: names.length, cut: names.filter((name) => name.scrollWidth > name.clientWidth).map((name) => name.textContent), first: names[0]?.textContent };
  });
  assert.ok(palette.rows > 10, JSON.stringify(palette));
  assert.deepEqual(palette.cut, [], "no palette name is cut");
  assert.ok(/^standardPalette\.[a-z]+\d+$/.test(palette.first), JSON.stringify(palette));
  // The sticky group heading hides the rows that scroll under it.
  const covered = await popover.evaluate((element) => {
    const list = element.querySelector(".named-list");
    const deep = element.querySelector('.named-row[data-name="env.color.text.muted"]');
    list.scrollTop += deep.getBoundingClientRect().top - list.getBoundingClientRect().top - 4;
    const heading = [...element.querySelectorAll(".named-group")].find((candidate) => {
      const box = candidate.getBoundingClientRect();
      return box.top <= list.getBoundingClientRect().top + 3 && box.bottom > list.getBoundingClientRect().top;
    });
    if (heading === undefined) { return JSON.stringify({ list: list.getBoundingClientRect().top, scroll: list.scrollTop, heads: [...element.querySelectorAll(".named-group")].map((g) => [g.textContent, g.getBoundingClientRect().top, g.getBoundingClientRect().bottom]) }); }
    const box = heading.getBoundingClientRect();
    // Nothing shows between the list's top edge and the heading.
    const hit = document.querySelector("nx-viewer").shadowRoot.elementFromPoint(box.left + box.width / 2, list.getBoundingClientRect().top + 0.5);
    return hit === heading || heading.contains(hit) ? "ok" : `${hit?.className}: ${hit?.textContent}`;
  });
  assert.equal(covered, "ok");
  if (process.env.NX_NAMED_SCREENSHOTS) {
    await page.screenshot({ path: `${process.env.NX_NAMED_SCREENSHOTS}/named-values-palette.png`, clip: await popover.boundingBox() });
  }
  await popover.evaluate((element) => {
    const marked = element.querySelector(".named-row.current");
    const list = element.querySelector(".named-list");
    list.scrollTop += marked.getBoundingClientRect().top - list.getBoundingClientRect().top - 60;
  });
  if (process.env.NX_NAMED_SCREENSHOTS) {
    const box = await popover.boundingBox();
    const at = await fill.boundingBox();
    const left = Math.max(0, Math.min(box.x, at.x) - 24);
    const top = Math.max(0, at.y - 40);
    await page.screenshot({ path: `${process.env.NX_NAMED_SCREENSHOTS}/named-values-dark.png`, clip: { x: left, y: top, width: Math.max(box.x + box.width, at.x + at.width) - left + 24, height: box.y + box.height - top + 16 } });
  }
  await page.keyboard.press("Escape");
  assert.equal(await popover.count(), 0);

  // In a narrow popover, a name that can't fit is cut in the middle: its end stays, the whole name
  // is its accessible name, and the lines under the list give it in full.
  await viewer.evaluate((element) => {
    const narrow = document.createElement("style");
    narrow.id = "narrow-named";
    narrow.textContent = ".popover.named { min-width: 0; max-width: 13rem; }";
    element.shadowRoot.append(narrow);
  });
  await row(`${tile}.fill`).locator(".reference-token").click();
  await popover.waitFor();
  const cut = await popover.evaluate((element) => {
    const option = [...element.querySelectorAll(".named-row")].find((candidate) => candidate.querySelector(".named-name")?.textContent.includes("\u2026"));
    if (option === undefined) { return JSON.stringify({ width: element.getBoundingClientRect().width, parent: element.parentElement?.className, longest: Math.max(...[...element.querySelectorAll(".named-name")].map((n) => n.scrollWidth)) }); }
    option.scrollIntoView({ block: "center" });
    return { shown: option.querySelector(".named-name").textContent, name: option.dataset.name, label: option.getAttribute("aria-label") };
  });
  assert.ok(typeof cut === "object", cut);
  assert.ok(cut.name.endsWith(cut.shown.split("\u2026").pop()), JSON.stringify(cut));
  assert.ok(cut.label.startsWith(cut.name), JSON.stringify(cut));
  await popover.locator(`.named-row[data-name="${cut.name}"]`).hover();
  assert.ok((await popover.locator(".named-doc").textContent()).startsWith(cut.name), "the detail line gives the whole name");
  await page.keyboard.press("Escape");
  await viewer.evaluate((element) => element.shadowRoot.getElementById("narrow-named").remove());

  assert.deepEqual(errors, []);
  console.log("PASS: named values: popover, filter, choose, undo, literal binding, unset property, swatches.");
} finally {
  await browser?.close();
  try { process.kill(-server.pid, "SIGTERM"); } catch {}
}
