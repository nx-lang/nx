/**
 * Writes `src/generated/`, the TypeScript types of the `@nx/agent` library that this package
 * publishes, from what `nxlang typegen @nx/agent --language typescript` generates. With `--check`
 * it writes nothing and fails when the checked-in files are not what it would write.
 *
 * One change is made to the generator's output: each relative import gains its `.js` extension
 * (`"./_nx"` becomes `"./_nx.js"`). The generator writes them without one, which a bundler
 * resolves and Node's own resolution (`moduleResolution: nodenext`) does not, and this package is
 * published for both. Nothing else differs, so the types are the generator's.
 *
 *   node scripts/generate-library-types.mjs          regenerate
 *   node scripts/generate-library-types.mjs --check  verify, as the test suite does
 */
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = resolve(packageRoot, "..", "..");
const generatedDir = join(packageRoot, "src", "generated");
const check = process.argv.includes("--check");

/** Every `from "./name"` of a generated module, with the extension Node's resolution needs. */
function withImportExtensions(source) {
  return source.replace(/(\bfrom\s+")(\.\/[^"]+?)(?<!\.js)(")/g, "$1$2.js$3");
}

const scratch = mkdtempSync(join(tmpdir(), "nx-agent-types-"));
try {
  const result = spawnSync(
    "cargo",
    ["run", "-q", "-p", "nx-cli", "--", "typegen", "@nx/agent", "--language", "typescript", "--output", scratch],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`nxlang typegen @nx/agent failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  }
  const generated = new Map(
    readdirSync(scratch)
      .sort()
      .map((name) => [name, withImportExtensions(readFileSync(join(scratch, name), "utf8"))]),
  );
  if (generated.size === 0) {
    throw new Error("nxlang typegen @nx/agent wrote no files");
  }

  if (!check) {
    rmSync(generatedDir, { recursive: true, force: true });
    mkdirSync(generatedDir, { recursive: true });
    for (const [name, source] of generated) {
      writeFileSync(join(generatedDir, name), source);
    }
    console.log(`Wrote ${[...generated.keys()].join(", ")} to src/generated`);
  } else {
    let checkedIn;
    try {
      checkedIn = readdirSync(generatedDir).sort();
    } catch {
      checkedIn = [];
    }
    const problems = [];
    for (const name of new Set([...generated.keys(), ...checkedIn])) {
      if (!generated.has(name)) {
        problems.push(`${name} is checked in and the generator does not write it`);
      } else if (!checkedIn.includes(name)) {
        problems.push(`${name} is generated and not checked in`);
      } else if (readFileSync(join(generatedDir, name), "utf8") !== generated.get(name)) {
        problems.push(`${name} is not what the generator writes`);
      }
    }
    if (problems.length > 0) {
      console.error(
        `src/generated is out of date:\n  ${problems.join("\n  ")}\nRun 'node scripts/generate-library-types.mjs' in packages/agent and commit the result.`,
      );
      process.exitCode = 1;
    }
  }
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
