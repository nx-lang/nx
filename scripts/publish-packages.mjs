/**
 * Publishes every npm tarball in a directory, in dependency order, skipping versions the registry
 * already has.
 *
 * `node scripts/publish-packages.mjs <dir> [--version <version>] [--dry-run]`
 *
 * Each tarball's manifest is read and checked (no `workspace:` specifier, public access, and the
 * given `--version` when there is one). The tarballs are then ordered so that a package follows
 * every `@nx-lang/*` package it depends on, including peers, and each is published with
 * `npm publish --access public --provenance`. A version already on the registry is skipped with a
 * warning rather than failed, so a repair run of a partial publish fills in what is missing and
 * touches nothing else. `--dry-run` prints the plan, checks the registry, and publishes nothing.
 */
import { execFileSync, spawnSync } from "node:child_process";
import { resolve } from "node:path";

import { inDependencyOrder, packedManifest, readTarballManifests } from "./workspace-packages.mjs";

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const versionIndex = args.indexOf("--version");
const version = versionIndex >= 0 ? args[versionIndex + 1] : undefined;
const dir = args.find((arg, index) => !arg.startsWith("--") && !(versionIndex >= 0 && index === versionIndex + 1));

if (!dir) {
  console.error("usage: node scripts/publish-packages.mjs <dir> [--version <version>] [--dry-run]");
  process.exit(2);
}

const tarballs = readTarballManifests(resolve(dir));
if (tarballs.length === 0) {
  console.error(`publish-packages: no .tgz files in ${dir}`);
  process.exit(1);
}

const problems = [];
for (const { file, manifest } of tarballs) {
  problems.push(
    ...packedManifest(manifest, version ?? manifest.version).map((problem) => `${file}: ${problem}`),
  );
}
if (problems.length > 0) {
  console.error(problems.map((problem) => `publish-packages: ${problem}`).join("\n"));
  process.exit(1);
}

function onRegistry(name, packageVersion) {
  const result = spawnSync("npm", ["view", `${name}@${packageVersion}`, "version"], { encoding: "utf8" });
  return result.status === 0 && result.stdout.trim() !== "";
}

let published = 0;
for (const { path, manifest } of inDependencyOrder(tarballs)) {
  const spec = `${manifest.name}@${manifest.version}`;
  if (onRegistry(manifest.name, manifest.version)) {
    console.log(`::warning::${spec} already exists on npm; skipping it as an idempotent retry.`);
    continue;
  }
  if (dryRun) {
    console.log(`would publish ${spec} from ${path}`);
    continue;
  }
  console.log(`publishing ${spec}`);
  execFileSync("npm", ["publish", path, "--access", "public", "--provenance"], { stdio: "inherit" });
  published += 1;
}

console.log(dryRun ? `dry run over ${tarballs.length} package(s)` : `published ${published} of ${tarballs.length} package(s)`);
