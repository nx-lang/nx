/**
 * Re-copies DrawnUI's source, demo pages, and shared assets into this app.
 *
 * Upstream is a private, unbuilt package, so there is nothing to depend on and the tree is
 * vendored instead. Local edits to the copy are allowed where they improve NX compatibility, so
 * the point of this script is less "stay in sync" than "record what was copied and from where":
 * it writes the upstream commit into src/drawnui/UPSTREAM.md, and a sync that changes the catalog
 * then shows up as a reviewable diff.
 *
 * Usage: npm run sync-drawnui [-- --source <path>]
 */
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const DEFAULT_SOURCE = join(homedir(), "src", "DrawnUi.React");

/** The trees copied, each `from` relative to the upstream root and `to` relative to this app. */
const COPIES = [
  { from: "src", to: "src/drawnui", what: "runtime source" },
  { from: "samples/demo/pages", to: "reference/demo-pages", what: "demo pages (reference only)" },
  { from: "samples/public/fonts", to: "public/fonts", what: "shared fonts" },
  { from: "samples/public/images", to: "public/images", what: "shared images" },
];

function parseSource(argv) {
  const flag = argv.indexOf("--source");
  if (flag !== -1) {
    const value = argv[flag + 1];
    if (value === undefined) {
      throw new Error("--source needs a path");
    }
    return resolve(value);
  }
  return resolve(process.env.DRAWNUI_SOURCE ?? DEFAULT_SOURCE);
}

function upstreamRevision(source) {
  const result = spawnSync("git", ["-C", source, "rev-parse", "HEAD"], { encoding: "utf8" });
  if (result.status !== 0) {
    return { commit: "unknown", dirty: false };
  }
  const status = spawnSync("git", ["-C", source, "status", "--porcelain"], { encoding: "utf8" });
  return {
    commit: result.stdout.trim(),
    dirty: status.status === 0 && status.stdout.trim().length > 0,
  };
}

function main() {
  const source = parseSource(process.argv.slice(2));
  if (!existsSync(source)) {
    throw new Error(
      `DrawnUI source not found at ${source}. Pass --source <path> or set DRAWNUI_SOURCE.`,
    );
  }

  for (const copy of COPIES) {
    const from = join(source, copy.from);
    if (!existsSync(from)) {
      throw new Error(`Expected ${from} to exist in the DrawnUI source tree.`);
    }
    const to = join(appRoot, copy.to);
    rmSync(to, { force: true, recursive: true });
    mkdirSync(dirname(to), { recursive: true });
    cpSync(from, to, { recursive: true });
    console.log(`copied ${copy.from} -> ${copy.to} (${copy.what})`);
  }

  const revision = upstreamRevision(source);
  const stamp = new Date().toISOString().slice(0, 10);
  writeFileSync(
    join(appRoot, "src/drawnui/UPSTREAM.md"),
    `# Vendored DrawnUI source

Copied by \`npm run sync-drawnui\`. Do not treat this tree as read-only: edits that improve NX
compatibility are expected, and \`docs/CATALOG.md\` records them. Re-running the sync overwrites
those edits, so re-apply them from that list.

| | |
|---|---|
| Upstream | \`${source}\` |
| Commit | \`${revision.commit}\`${revision.dirty ? " (working tree was dirty)" : ""} |
| Copied | ${stamp} |

Trees copied:

${COPIES.map((copy) => `- \`${copy.from}\` → \`${copy.to}\` — ${copy.what}`).join("\n")}
`,
  );
  console.log(`recorded upstream commit ${revision.commit}${revision.dirty ? " (dirty)" : ""}`);
}

main();
