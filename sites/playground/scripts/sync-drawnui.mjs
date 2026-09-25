/**
 * Copies DrawnUI's shared assets and demo pages into this app from the upstream tag matching the
 * pinned `drawnui-react` package.
 *
 * The runtime is the npm package. It ships only its `dist`, so the fonts, images, Lottie files,
 * sprite sheets and shaders the examples load, and the demo pages they were ported from, still come
 * from the upstream repository. They are read at the tag `v<pin>` with `git archive`, so they
 * match the code that draws them. The upstream working tree and its `HEAD` are never touched.
 * The tag and commit are written to docs/UPSTREAM.md, and a sync that changes an asset shows up as
 * a reviewable diff.
 *
 * Usage: pnpm run sync-drawnui [-- --source <path>]
 */
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const DEFAULT_SOURCE = join(homedir(), "src", "DrawnUi.React");

/** The trees copied, each `from` relative to the upstream root and `to` relative to this app. */
const COPIES = [
  { from: "samples/demo/pages", to: "reference/demo-pages", what: "demo pages (reference only)" },
  { from: "samples/public/fonts", to: "public/fonts", what: "shared fonts" },
  { from: "samples/public/images", to: "public/images", what: "shared images" },
  { from: "samples/public/lottie", to: "public/lottie", what: "Lottie animations" },
  { from: "samples/public/anims", to: "public/anims", what: "sprite sheets" },
  { from: "samples/public/shaders", to: "public/shaders", what: "SkSL shaders" },
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

function git(source, args, options = {}) {
  const result = spawnSync("git", ["-C", source, ...args], { maxBuffer: 1 << 30, ...options });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(" ")} failed in ${source}:\n${result.stderr}`);
  }
  return result.stdout;
}

/** The commit of the tag for the pinned package, or a failure that says how to get it. */
function taggedCommit(source, tag) {
  const result = spawnSync("git", ["-C", source, "rev-parse", "--verify", "--quiet", `refs/tags/${tag}^{commit}`], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(
      `${source} has no tag ${tag}, the release of the pinned drawnui-react. Run \`git -C ${source} fetch --tags\` and sync again.`,
    );
  }
  return result.stdout.trim();
}

function main() {
  const source = parseSource(process.argv.slice(2));
  if (!existsSync(source)) {
    throw new Error(`DrawnUI source not found at ${source}. Pass --source <path> or set DRAWNUI_SOURCE.`);
  }
  const pin = JSON.parse(readFileSync(join(appRoot, "package.json"), "utf8")).dependencies["drawnui-react"];
  const tag = `v${pin}`;
  const commit = taggedCommit(source, tag);

  const staging = mkdtempSync(join(tmpdir(), "sync-drawnui-"));
  try {
    const archive = git(source, ["archive", "--format=tar", tag, ...COPIES.map((copy) => copy.from)]);
    const untar = spawnSync("tar", ["-x", "-C", staging], { input: archive });
    if (untar.status !== 0) {
      throw new Error(`tar could not unpack the archive of ${tag}:\n${untar.stderr}`);
    }
    for (const copy of COPIES) {
      const from = join(staging, copy.from);
      if (!existsSync(from)) {
        throw new Error(`Expected ${copy.from} to exist in DrawnUI at ${tag}.`);
      }
      const to = join(appRoot, copy.to);
      rmSync(to, { force: true, recursive: true });
      mkdirSync(dirname(to), { recursive: true });
      cpSync(from, to, { recursive: true });
      console.log(`copied ${copy.from} -> ${copy.to} (${copy.what})`);
    }
  } finally {
    rmSync(staging, { force: true, recursive: true });
  }

  writeFileSync(
    join(appRoot, "docs/UPSTREAM.md"),
    `# DrawnUI assets

Copied by \`pnpm run sync-drawnui\` from the DrawnUI release the site pins. The runtime itself is the
\`drawnui-react\` npm package. These trees are what that package does not ship. Do not edit them:
the next sync overwrites them.

| | |
|---|---|
| Package | \`drawnui-react\` ${pin} |
| Tag | \`${tag}\` |
| Commit | \`${commit}\` |

Trees copied:

${COPIES.map((copy) => `- \`${copy.from}\` → \`${copy.to}\` — ${copy.what}`).join("\n")}
`,
  );
  console.log(`recorded ${tag} (${commit})`);
}

main();
