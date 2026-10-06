import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";

import { disposeCompiled, sourceDiagnostics } from "./nx-support.js";
import { packageRoot, repositoryRoot } from "./support.js";

after(disposeCompiled);

const readme = readFileSync(join(packageRoot, "README.md"), "utf8");

// The website's page for hosts, which shows the package in use. Its NX samples are checked by the
// website's own test; its TypeScript samples are compiled here, beside the README's.
const hostPagePath = join(repositoryRoot, "sites", "website", "src", "content", "docs", "reference", "libraries", "agent-hosts.md");

/** The fenced blocks of one language in `text`, each with the line it begins on. */
function blocks(language: string, text = readme): { readonly line: number; readonly code: string }[] {
  const found: { line: number; code: string }[] = [];
  const lines = text.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    if (lines[index] !== `\`\`\`${language}`) {
      continue;
    }
    const end = lines.indexOf("```", index + 1);
    assert.ok(end > index, `the block at line ${index + 1} is not closed`);
    found.push({ line: index + 1, code: lines.slice(index + 1, end).join("\n") });
    index = end;
  }
  return found;
}

test("the README and the manifest say the package is unstable", () => {
  assert.match(readme, /^> \*\*Unstable\.\*\*/m);
  const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")) as { description: string };
  assert.match(manifest.description, /^Unstable\./);
});

test("no sample puts the text of an error it caught into a tool error, which the AI SDK adapter may give the model", () => {
  // The adapter keeps the message of an error whose code is the host's own. A sample that builds
  // one from a caught error would teach hosts to send the model whatever `fetch` threw.
  for (const [name, text] of [["README.md", readme], ["agent-hosts.md", readFileSync(hostPagePath, "utf8")]] as const) {
    for (const sample of blocks("ts", text)) {
      const built = sample.code.match(/new NxAgentToolError\([^;]*;/g) ?? [];
      for (const call of built) {
        assert.doesNotMatch(call, /String\(\s*error\s*\)|error\.message|\$\{\s*error\b/, `${name}, line ${sample.line}: ${call}`);
      }
    }
  }
});

test("every fenced block of the README is TypeScript or NX, so every sample is checked", () => {
  const fences = readme.split("\n").filter((line) => line.startsWith("```") && line !== "```");
  assert.deepEqual([...new Set(fences)].sort(), ["```nx", "```ts"]);
  assert.equal(fences.length, blocks("ts").length + blocks("nx").length);
});

/** Compiles each sample as a module of its own, in a project that has the built package installed. */
function compileSamples(samples: readonly { readonly line: number; readonly code: string }[], name: string): void {
  const consumer = mkdtempSync(join(tmpdir(), "nx-agent-readme-"));
  try {
    // The package under its own name, and what a host that follows the README installs beside it.
    mkdirSync(join(consumer, "node_modules", "@nx-lang"), { recursive: true });
    symlinkSync(packageRoot, join(consumer, "node_modules", "@nx-lang", "agent"), "dir");
    for (const dependency of ["@nx-lang/ir-runtime", "@nx-lang/sdk-wasm", "ai"]) {
      symlinkSync(realpathSync(join(packageRoot, "node_modules", dependency)), join(consumer, "node_modules", dependency), "dir");
    }
    writeFileSync(join(consumer, "package.json"), JSON.stringify({ name: "consumer", private: true, type: "module" }));
    const files = samples.map((sample) => {
      // Named by the line the sample begins on, so a compiler error says which one it is in.
      const file = `${name}-line-${sample.line}.ts`;
      writeFileSync(join(consumer, file), `${sample.code}\n`);
      return file;
    });
    const result = spawnSync(
      join(packageRoot, "node_modules", ".bin", "tsc"),
      [
        "--noEmit",
        "--strict",
        "--exactOptionalPropertyTypes",
        "--noUncheckedIndexedAccess",
        "--skipLibCheck",
        "--target",
        "es2022",
        "--lib",
        "es2022,dom,dom.iterable",
        "--module",
        "esnext",
        "--moduleResolution",
        "bundler",
        ...files,
      ],
      { cwd: consumer, encoding: "utf8" },
    );
    assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  } finally {
    rmSync(consumer, { recursive: true, force: true });
  }
}

test("every TypeScript sample of the README compiles against the built package", () => {
  const samples = blocks("ts");
  assert.ok(samples.length >= 6, `only ${samples.length} samples were found`);
  compileSamples(samples, "readme");
});

test("every TypeScript sample of the website's page for hosts compiles against the built package", () => {
  let page: string;
  try {
    page = readFileSync(hostPagePath, "utf8");
  } catch {
    assert.fail(`The website's page for agent hosts is not at ${hostPagePath}. If it moved, point this test at it.`);
  }
  const samples = blocks("ts", page);
  assert.ok(samples.length >= 2, `only ${samples.length} samples were found`);
  compileSamples(samples, "agent-hosts");
});

test("every NX sample of the README compiles", () => {
  const samples = blocks("nx");
  assert.ok(samples.length >= 1);
  for (const sample of samples) {
    assert.deepEqual(sourceDiagnostics(sample.code), [], `the NX sample at line ${sample.line}`);
  }
});
