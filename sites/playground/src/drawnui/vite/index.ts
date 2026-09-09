/**
 * Vite helpers for DrawnUi.React apps: `import { drawnUiStatic } from "drawnui-react/vite"`.
 * Build-time only; the runtime bundle never imports this module and the frame loop knows nothing about it.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { Plugin, ResolvedConfig } from "vite";

export interface DrawnUiStaticOptions {
  /**
   * Pages to generate, as URL hashes / paths appended to the site root ("" = the root page, "#/shapes"...).
   * Default: the root page plus every page a button on the root page navigates to (discovered by clicking it
   * in the headless browser and reading the new location).
   */
  routes?: string[];
  /** Id of the element the app mounts into; the generated HTML is placed right after it (or at `<!-- drawnui-static -->` if present). Default "root". */
  container?: string;
  /** Chrome executable. Default: the installed Google Chrome (playwright-core `channel: "chrome"`), or $CHROME_PATH. */
  executablePath?: string;
  /** Milliseconds the accessibility tree must stay unchanged before a page is read (the engine rebuilds it at most every 300 ms). Default 800. */
  settleMs?: number;
  /** Text of the generated top heading. Default: the document <title>. */
  heading?: string;
  /** Look of the block: `style` (CSS, default DEFAULT_STATIC_STYLE, `false` = none). */
  render?: RenderStaticOptions;
}

/** One accessible control as read from the running app (the engine's accessibility snapshot, exposed as DOM). */
export interface StaticNode { Role: string; Label: string; Hint?: string; Href?: string }

/**
 * Generates visible, semantic HTML for crawlers and AI agents from the app's own accessibility tree, at build time.
 *
 * After `vite build` it serves the build (`vite preview`), opens it in headless Chrome, lets the engine measure,
 * arrange and draw, reads the accessibility snapshot of every page (root + the pages its buttons open, or the
 * `routes` you list) and writes ordinary HTML into `dist/index.html`: a heading becomes a heading, a button becomes
 * a link to the page it opened, a label becomes a paragraph, a form control becomes a list item naming it. The
 * overlay's own markup (transparent text over an `aria-hidden` canvas) is deliberately NOT copied — in a static
 * file that reads as hidden text; this output is plain flow content, nothing hidden, nothing collapsed.
 *
 * Placement: right AFTER the mount element (`#root` by default), or wherever you put `<!-- drawnui-static -->` in
 * index.html. With the usual layout (the mount element filling the viewport) it sits below the fold: a visitor
 * sees the app area loading, the text is a scroll away and genuinely visible. Removal: the first `<Canvas>` that
 * mounts removes every `[data-drawnui-static]` element once its engine has drawn frame 1 (one querySelectorAll,
 * nothing in the frame loop; an app built without the plugin finds nothing). If the app never boots it stays.
 * Removing it only shrinks the document below the fold; the mount element keeps its size, so nothing above the
 * fold moves — except that a page which scrolled only because of the block loses its scrollbar (classic scrollbars
 * on Windows/Linux then give the canvas those ~15 px back — set `html { scrollbar-gutter: stable }` so the
 * canvas has the same width before and after boot; the demo does). If the mount element is a box inside a longer page, your own content below it moves
 * up by the block's height when it goes; place the marker comment at the end of the body to avoid that.
 *
 * Two things to know:
 * - Verify with `curl` or view-source, not DevTools after boot: the app removed it.
 * - Crawlers that do not run JavaScript (GPTBot, ClaudeBot, CCBot, curl, link previews) read this HTML.
 *   Googlebot renders JavaScript: it sees the booted page, i.e. the accessibility overlay (roles, labels, text).
 *   Both come from the same accessibility tree, so they cannot drift — but a control without an
 *   `AccessibilityRole` is invisible to both.
 *
 * Needs `playwright-core` (dev dependency of your app) and a Chrome to drive; GitHub's ubuntu runners have one.
 * Usage: `plugins: [react(), drawnUiStatic()]`.
 */
export function drawnUiStatic(options: DrawnUiStaticOptions = {}): Plugin {
  const container = options.container ?? "root";
  const settleMs = options.settleMs ?? 800;
  let config: ResolvedConfig;
  return {
    name: "drawnui-static",
    apply: "build",
    configResolved(c) { config = c; },
    async closeBundle() {
      const file = resolve(config.root, config.build.outDir, "index.html");
      const html = readFileSync(file, "utf8");
      const marker = "<!-- drawnui-static -->";
      const slot = html.includes(marker) ? marker : new RegExp(`<div\\b[^>]*\\bid="${container}"[^>]*>\\s*</div>`).exec(html)?.[0];
      if (!slot) { config.logger.warn(`drawnui-static: neither ${marker} nor an empty <div id="${container}"> in ${file}, nothing generated`); return; }

      const { preview } = await import("vite");
      const server = await preview({ configFile: config.configFile, root: config.root, mode: config.mode, logLevel: "warn", preview: { port: 0, strictPort: false, open: false } });
      const origin = server.resolvedUrls?.local[0]?.replace(/\/$/, "") ?? "";
      const base = config.base.replace(/\/$/, "");
      const { chromium } = await import("playwright-core");
      const browser = await chromium.launch(options.executablePath ?? process.env.CHROME_PATH ? { executablePath: options.executablePath ?? process.env.CHROME_PATH } : { channel: "chrome" });
      try {
        const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
        const errors: string[] = [];
        page.on("pageerror", (e) => errors.push(String(e)));
        const url = (route: string) => `${origin}${base}/${route}`;

        // the engine rebuilds the accessibility snapshot at most every 300 ms and only after a drawn frame:
        // wait until the overlay has stopped changing
        const readTree = async (): Promise<StaticNode[]> => {
          let last = "", stableSince = Date.now(), started = Date.now();
          for (;;) {
            const nodes = await page.evaluate(() => [...document.querySelectorAll<HTMLElement>(".drawnui-a11y-node")].map((e) => ({
              Role: e.getAttribute("role") ?? "", Label: e.getAttribute("aria-label") ?? e.textContent ?? "", Hint: e.getAttribute("title") ?? undefined,
            })));
            const key = JSON.stringify(nodes);
            if (key !== last) { last = key; stableSince = Date.now(); }
            else if (nodes.length && Date.now() - stableSince >= settleMs) return nodes;
            if (Date.now() - started > 20000) return nodes;
            await page.waitForTimeout(150);
          }
        };

        await page.goto(url(""));
        const root = await readTree();
        const pages = new Map<string, StaticNode[]>([["", root]]);
        if (options.routes) {
          for (const r of options.routes) { if (r === "") continue; await page.goto(url(r)); pages.set(r, await readTree()); }
        } else {
          // a button becomes a link to wherever it takes the user: click it, read the new location, come back
          for (let i = 0; i < root.length; i++) {
            if (root[i].Role !== "button") continue;
            await page.goto(url(""));
            await readTree();
            // activate through the overlay node (routed to the control as a Tapped), so buttons below the fold work too
            const activated = await page.evaluate((i) => { const el = document.querySelectorAll<HTMLElement>(".drawnui-a11y-node")[i]; if (!el) return false; el.click(); return true; }, i);
            if (!activated) continue;
            await page.waitForTimeout(400);
            const route = await page.evaluate(() => location.hash);
            if (!route || pages.has(route)) continue;
            root[i].Href = route;
            pages.set(route, await readTree());
          }
        }
        if (errors.length) config.logger.warn(`drawnui-static: page errors while generating: ${errors.join("; ")}`);

        const heading = options.heading ?? /<title>([^<]*)<\/title>/.exec(html)?.[1] ?? "";
        const markup = renderStatic(heading, pages, options.render);
        writeFileSync(file, html.replace(slot, () => (slot === marker ? markup : `${slot}\n${markup}`)));
        config.logger.info(`drawnui-static: ${pages.size} page(s), ${[...pages.values()].reduce((n, p) => n + p.length, 0)} nodes after #${container} in ${file}`);
      } finally {
        await browser.close();
        await server.close();
      }
    },
  };
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/**
 * Default look of the generated block, shipped inline so every app gets a calm "loading its contents" page out of
 * the box. Everything inherits the page's font and text color (set `body { color }` for a dark site); surfaces are
 * translucent so they read on light and dark backgrounds. Override by styling `.drawnui-static` (your stylesheet
 * comes later in the document and wins), or pass `style: false` to ship no default style at all.
 */
export const DEFAULT_STATIC_STYLE = `
.drawnui-static{max-width:820px;margin:0 auto;padding:32px 24px 48px;line-height:1.5;color:inherit}
.drawnui-static h1{font-size:28px;font-weight:600;margin:0 0 4px}
.drawnui-static h2{font-size:20px;font-weight:600;margin:28px 0 8px}
.drawnui-static h3{font-size:16px;font-weight:600;margin:16px 0 4px;opacity:.9}
.drawnui-static p{margin:0 0 8px;opacity:.85}
.drawnui-static ul{list-style:none;margin:0;padding:0;display:grid;gap:8px}
.drawnui-static li{padding:10px 14px;border-radius:8px;background:rgba(128,128,128,.14)}
.drawnui-static li a{color:inherit;text-decoration:none;display:block}
.drawnui-static li b{font-weight:600;margin-right:6px}
`;

export interface RenderStaticOptions {
  /** CSS placed before the block; `false` ships none. Default DEFAULT_STATIC_STYLE. */
  style?: string | false;
}

/**
 * Accessibility nodes -> visible semantic HTML: one <section> per page (id = route), roles map to ordinary elements.
 * Everything is open flow content; the block and its <style> carry data-drawnui-static so the app can remove them.
 */
export function renderStatic(heading: string, pages: Map<string, StaticNode[]>, options: RenderStaticOptions = {}): string {
  const style = options.style === undefined ? DEFAULT_STATIC_STYLE : options.style;
  const out: string[] = [];
  if (style) out.push(`<style data-drawnui-static>${style}</style>`);
  out.push(`<div class="drawnui-static" data-drawnui-static>`);
  if (heading) out.push(`<h1>${esc(heading)}</h1>`);
  for (const [route, nodes] of pages) {
    const titleNode = nodes.find((n) => n.Role === "heading" && n.Label.trim());
    const title = titleNode?.Label.trim() ?? route.replace(/^#\/?/, "");
    const id = route ? ` id="${esc(route.replace(/^#\/?/, ""))}"` : "";
    out.push(`<section${id}>`);
    if (route) out.push(`<h2>${esc(title)}</h2>`);
    let list: string[] = [];
    const flush = () => { if (list.length) { out.push(`<ul>${list.join("")}</ul>`); list = []; } };
    for (const n of nodes) {
      const label = n.Label.trim();
      if (!label || (n === titleNode && route)) continue;
      const hint = n.Hint?.trim();
      switch (n.Role) {
        case "heading": flush(); out.push(route ? `<h3>${esc(label)}</h3>` : `<h2>${esc(label)}</h2>`); break;
        case "button": case "link": case "tab": case "menuitem":
          list.push(`<li>${n.Href ? `<a href="${esc(n.Href)}"><b>${esc(label)}</b>` : `<b>${esc(label)}</b>`}${hint ? ` ${esc(hint)}` : ""}${n.Href ? "</a>" : ""}</li>`); break;
        case "text": case "status": case "alert": flush(); out.push(`<p>${esc(label)}</p>`); break;
        case "img": break;
        default: list.push(`<li>${esc(label)}${hint ? ` — ${esc(hint)}` : ""} <small>(${esc(n.Role)})</small></li>`);
      }
    }
    flush();
    out.push(`</section>`);
  }
  out.push(`</div>`);
  return out.join("\n");
}
