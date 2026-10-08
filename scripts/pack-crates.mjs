/**
 * Packs the crates that go to crates.io at one version and checks each package.
 *
 * `node scripts/pack-crates.mjs <version> <output-dir>` sets the workspace's version to `<version>`
 * and the requirements the runtime crates have on each other to exactly `<version>`, runs
 * `cargo package` for `nx-ir`, `nx-value` and `nx-ir-runtime`, writes the `.crate` files to
 * `<output-dir>`, and then restores the manifest and the lock file, so a local run leaves the tree
 * as it found it. It is the crates' counterpart of `pack-packages.mjs` and takes the same version.
 *
 * `cargo package` builds each crate from its packaged contents, so this is also the check that a
 * packaged crate builds without the checkout. Each packed manifest is then checked to carry the
 * crate's name and `<version>` and to pin the other runtime crates at exactly `<version>`.
 *
 * The same commit packed twice at the same version gives the same bytes: the publish workflow
 * relies on that to prove that what it publishes is the file attached to the release.
 */
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { isSemanticVersion, packCrates, packedCrateProblems, sha256 } from "./runtime-crates.mjs";

const repositoryRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const [version, outputArgument] = process.argv.slice(2);

if (!version || !outputArgument) {
  console.error("usage: node scripts/pack-crates.mjs <version> <output-dir>");
  process.exit(2);
}
if (!isSemanticVersion(version)) {
  console.error(`pack-crates: '${version}' is not a semantic version.`);
  process.exit(2);
}

const packed = await packCrates(repositoryRoot, version, resolve(outputArgument));

const problems = packed.flatMap(({ name, file, path }) =>
  packedCrateProblems(path, name, version).map((problem) => `${file}: ${problem}`),
);
if (problems.length > 0) {
  console.error(problems.map((problem) => `pack-crates: ${problem}`).join("\n"));
  process.exit(1);
}

for (const { name, file, path } of packed) {
  console.log(`packed ${name}@${version} -> ${file} sha256:${sha256(path)}`);
}
