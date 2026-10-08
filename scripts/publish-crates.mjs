/**
 * Publishes the runtime crates attached to a release to crates.io, in dependency order, skipping
 * versions the registry already has, and proves that what it publishes is what was attached.
 *
 * `node scripts/publish-crates.mjs <assets-dir> --version <version> [--dry-run]`
 *
 * Cargo has no command that uploads an existing `.crate` file: `cargo publish` packages the crate
 * again from the sources. So this runs from a checkout of the release's tag and holds the result
 * to the reviewed files on both sides of the upload:
 *
 * 1. Each attached `.crate` is checked to be the crate it is named for, at `<version>`, pinning the
 *    other runtime crates at exactly `<version>`.
 * 2. The crates are packaged again here, at `<version>`, and the SHA-256 of each is compared with
 *    the attached file's. Any difference stops the run before anything is published.
 * 3. A version crates.io already has is skipped as an idempotent retry, after its recorded
 *    checksum is compared with the attached file's. The others are published with
 *    `cargo publish`, a crate after the crates it depends on; Cargo waits for the index to list
 *    each before returning.
 * 4. The checksum crates.io records for each published version is compared with the attached
 *    file's. A difference fails the run: the version is public and is not the reviewed file.
 *
 * `--dry-run` does steps 1 and 2, reads the registry, prints what step 3 would do, and publishes
 * nothing. Publishing reads the registry token from `CARGO_REGISTRY_TOKEN`, as Cargo does.
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  atReleaseVersion,
  crateFile,
  isSemanticVersion,
  packageCrates,
  packedCrateProblems,
  runtimeCrates,
  sha256,
} from "./runtime-crates.mjs";

const repositoryRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const versionIndex = args.indexOf("--version");
const version = versionIndex >= 0 ? args[versionIndex + 1] : undefined;
const dir = args.find((arg, index) => !arg.startsWith("--") && !(versionIndex >= 0 && index === versionIndex + 1));

if (!dir || !version) {
  console.error("usage: node scripts/publish-crates.mjs <assets-dir> --version <version> [--dry-run]");
  process.exit(2);
}
if (!isSemanticVersion(version)) {
  console.error(`publish-crates: '${version}' is not a semantic version.`);
  process.exit(2);
}

function fail(message) {
  console.error(`publish-crates: ${message}`);
  process.exit(1);
}

const assets = runtimeCrates.map((name) => {
  const file = crateFile(name, version);
  return { name, file, path: join(resolve(dir), file) };
});

const problems = assets.flatMap(({ name, file, path }) =>
  existsSync(path)
    ? packedCrateProblems(path, name, version).map((problem) => `${file}: ${problem}`)
    : [`${file} is not in ${dir}`],
);
if (problems.length > 0) {
  fail(problems.join("\npublish-crates: "));
}
for (const asset of assets) {
  asset.sha256 = sha256(asset.path);
}

/**
 * The SHA-256 crates.io records for a version of a crate, or `undefined` when it has no such
 * version. Any other answer is an error: a registry that cannot be read is not a registry without
 * the version.
 */
async function registryChecksum(name) {
  const response = await fetch(`https://crates.io/api/v1/crates/${name}/${version}`, {
    headers: { "User-Agent": "nx-release-scripts (https://github.com/nx-lang/nx)", Accept: "application/json" },
  });
  if (response.status === 404) {
    return undefined;
  }
  if (!response.ok) {
    throw new Error(`crates.io answered ${response.status} for ${name}@${version}`);
  }
  const checksum = (await response.json())?.version?.checksum;
  if (typeof checksum !== "string") {
    throw new Error(`crates.io gave no checksum for ${name}@${version}`);
  }
  return checksum;
}

/** The registry's checksum for a version that was just published, which its API may list a moment later. */
async function publishedChecksum(name) {
  for (let attempt = 0; attempt < 12; attempt += 1) {
    const checksum = await registryChecksum(name);
    if (checksum !== undefined) {
      return checksum;
    }
    await new Promise((done) => setTimeout(done, 5000));
  }
  throw new Error(`crates.io does not list ${name}@${version} after publishing it`);
}

const scratch = mkdtempSync(join(tmpdir(), "nx-publish-crates-"));
try {
  await atReleaseVersion(repositoryRoot, version, async () => {
    // What `cargo publish` would upload from this checkout, compared with what was reviewed.
    const rebuilt = packageCrates(repositoryRoot, version, scratch);
    const different = rebuilt.filter(({ name, path }) => sha256(path) !== assets.find((asset) => asset.name === name).sha256);
    if (different.length > 0) {
      throw new Error(
        `packaged from this checkout, ${different.map(({ file }) => file).join(", ")} ` +
          `${different.length === 1 ? "is not the file" : "are not the files"} attached to the release; ` +
          `nothing was published. Run this from the release's tag, with the toolchain rust-toolchain.toml names.`,
      );
    }
    console.log(`the ${assets.length} crates package from this checkout to the attached files, byte for byte`);

    let published = 0;
    for (const { name, sha256: reviewed } of assets) {
      const spec = `${name}@${version}`;
      const existing = await registryChecksum(name);
      if (existing !== undefined) {
        if (existing !== reviewed) {
          throw new Error(`crates.io already has ${spec} with checksum ${existing}, which is not the attached file (${reviewed}).`);
        }
        console.log(`::warning::${spec} already exists on crates.io with the attached file's checksum; skipping it as an idempotent retry.`);
        continue;
      }
      if (dryRun) {
        console.log(`would publish ${spec} sha256:${reviewed}`);
        continue;
      }
      console.log(`publishing ${spec}`);
      // The tree holds the release version this script wrote, so Cargo is told to allow that.
      execFileSync("cargo", ["publish", "-p", name, "--allow-dirty"], { cwd: repositoryRoot, stdio: "inherit" });
      const recorded = await publishedChecksum(name);
      if (recorded !== reviewed) {
        throw new Error(`crates.io recorded checksum ${recorded} for ${spec}, which is not the attached file (${reviewed}). The version is public: yank it and release a higher one.`);
      }
      console.log(`published ${spec}; crates.io serves the attached file (sha256:${recorded})`);
      published += 1;
    }
    console.log(dryRun ? `dry run over ${assets.length} crate(s)` : `published ${published} of ${assets.length} crate(s)`);
  });
} catch (error) {
  // Not `fail`: the manifest and the lock file are put back on the way out, which an exit here
  // would skip.
  console.error(`publish-crates: ${error.message}`);
  process.exitCode = 1;
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
