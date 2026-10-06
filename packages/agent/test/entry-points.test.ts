import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { builtinModules } from "node:module";
import { dirname, join, relative } from "node:path";
import { test } from "node:test";

import ts from "typescript";

import { packageRoot } from "./support.js";

const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")) as {
  exports: Record<string, { types: string; import: string }>;
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
};

/**
 * Every module a built entry point loads from outside the package, with the built file that asks
 * for it: the entry's file and each file it reaches through a relative specifier. The specifiers
 * are read by TypeScript's own scanner, which sees `import`, `export ... from` and `import()` and
 * nothing in a comment or a string.
 */
function externalImports(entry: string): Map<string, string> {
  const external = new Map<string, string>();
  const seen = new Set<string>();
  const pending = [join(packageRoot, manifest.exports[entry]!.import)];
  while (pending.length > 0) {
    const file = pending.pop()!;
    if (seen.has(file)) {
      continue;
    }
    seen.add(file);
    for (const { fileName } of ts.preProcessFile(readFileSync(file, "utf8"), true, true).importedFiles) {
      if (fileName.startsWith(".")) {
        pending.push(join(dirname(file), fileName));
      } else if (!external.has(fileName)) {
        external.set(fileName, relative(packageRoot, file));
      }
    }
  }
  return external;
}

function isNodeBuiltin(specifier: string): boolean {
  return specifier.startsWith("node:") || builtinModules.includes(specifier);
}

/** What each entry point may load from outside the package at run time, and nothing else. */
const allowed: Readonly<Record<string, readonly string[]>> = {
  ".": [],
  "./normalize": [],
  "./execute": ["@nx-lang/ir-runtime"],
  "./ai-sdk": ["ai"],
};

test("the package declares its four entry points, each with types and an import", () => {
  assert.deepEqual(Object.keys(manifest.exports), [".", "./normalize", "./execute", "./ai-sdk"]);
  for (const [entry, target] of Object.entries(manifest.exports)) {
    assert.deepEqual(Object.keys(target), ["types", "import"], entry);
    readFileSync(join(packageRoot, target.import));
    readFileSync(join(packageRoot, target.types));
  }
});

for (const entry of Object.keys(allowed)) {
  test(`'${entry}' loads nothing from outside the package but ${allowed[entry]!.length === 0 ? "nothing at all" : allowed[entry]!.join(", ")}`, () => {
    const external = externalImports(entry);
    for (const [specifier, file] of external) {
      assert.equal(isNodeBuiltin(specifier), false, `${file} imports the Node built-in '${specifier}'`);
      assert.notEqual(specifier, "@nx-lang/sdk-wasm", `${file} imports the compiler`);
      assert.equal(specifier.endsWith(".wasm"), false, `${file} imports a WebAssembly module`);
      assert.ok(allowed[entry]!.includes(specifier), `${file} imports '${specifier}', which '${entry}' may not load`);
    }
    // And it does load what it is there for, so the walk above read real imports.
    assert.deepEqual([...external.keys()].sort(), [...allowed[entry]!].sort());
  });
}

test("only the AI SDK entry point imports ai, which is an optional peer", () => {
  for (const entry of [".", "./normalize", "./execute"]) {
    assert.equal(externalImports(entry).has("ai"), false, entry);
  }
  assert.equal(externalImports("./ai-sdk").has("ai"), true);
  assert.deepEqual(Object.keys(manifest.peerDependencies ?? {}), ["ai"]);
  assert.equal(manifest.peerDependenciesMeta?.["ai"]?.optional, true);
  assert.equal("ai" in (manifest.dependencies ?? {}), false);
  assert.deepEqual(Object.keys(manifest.dependencies ?? {}), ["@nx-lang/ir-runtime"]);
});
