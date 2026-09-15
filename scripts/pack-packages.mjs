/**
 * Packs every publishable workspace package at one version and checks each tarball.
 *
 * `node scripts/pack-packages.mjs <version> <output-dir>` sets every publishable package's version to
 * `<version>` in its `package.json`, runs `pnpm pack` on each — which rewrites `workspace:*`
 * references to the sibling's version, that is, to `<version>` — writes the tarballs to
 * `<output-dir>`, and then restores the manifests it changed, so a local run leaves the tree as it
 * found it. Each tarball's manifest is checked to carry `<version>`, no `workspace:` specifier, and
 * `@nx-lang/*` dependencies pinned to the same version.
 *
 * The publishable packages are the workspace members that are not `private`. The editor-assets
 * package under `src/vscode` is not a workspace member and has its own packaging script.
 *
 * Every package must already be built (`pnpm -r build`): `pnpm pack` runs each package's `prepack`,
 * which rebuilds from what is there, and the tarball is what the build left in `dist/`.
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { publishablePackages, packedManifest, readTarballManifests } from "./workspace-packages.mjs";

const repositoryRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const [version, outputArgument] = process.argv.slice(2);

if (!version || !outputArgument) {
  console.error("usage: node scripts/pack-packages.mjs <version> <output-dir>");
  process.exit(2);
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`pack-packages: '${version}' is not a semantic version.`);
  process.exit(2);
}

const outputDir = resolve(outputArgument);
mkdirSync(outputDir, { recursive: true });

const packages = publishablePackages(repositoryRoot);
const originals = new Map();

try {
  for (const { root, manifest } of packages) {
    const manifestPath = join(root, "package.json");
    originals.set(manifestPath, readFileSync(manifestPath, "utf8"));
    writeFileSync(manifestPath, `${JSON.stringify({ ...manifest, version }, null, 2)}\n`);
  }

  for (const { root, manifest } of packages) {
    execFileSync("pnpm", ["pack", "--pack-destination", outputDir], { cwd: root, stdio: "inherit" });
    const expected = `${manifest.name.replace("@", "").replace("/", "-")}-${version}.tgz`;
    if (!existsSync(join(outputDir, expected))) {
      const produced = readdirSync(outputDir).filter((name) => name.endsWith(".tgz"));
      throw new Error(`pnpm pack of ${manifest.name} did not produce ${expected}; found ${produced.join(", ")}`);
    }
  }
} finally {
  for (const [manifestPath, text] of originals) {
    writeFileSync(manifestPath, text);
  }
}

// What went into each tarball is what a consumer gets: check the packed manifests, not the sources.
const packed = readTarballManifests(outputDir);
const names = new Set(packages.map(({ manifest }) => manifest.name));
const problems = [];
for (const { file, manifest } of packed) {
  if (!names.has(manifest.name)) {
    problems.push(`${file}: ${manifest.name} is not a publishable workspace package`);
    continue;
  }
  names.delete(manifest.name);
  problems.push(...packedManifest(manifest, version).map((problem) => `${file}: ${problem}`));
}
for (const name of names) {
  problems.push(`${name} was not packed`);
}
if (problems.length > 0) {
  console.error(problems.map((problem) => `pack-packages: ${problem}`).join("\n"));
  process.exit(1);
}

for (const { file, manifest } of packed) {
  console.log(`packed ${manifest.name}@${manifest.version} -> ${file}`);
}
