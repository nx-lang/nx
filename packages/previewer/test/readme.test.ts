import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { describe, it } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import ts from "typescript";

import { images } from "./fixtures.js";

const packageRoot = fileURLToPath(new URL("../../", import.meta.url));
const readme = readFileSync(new URL("README.md", pathToFileURL(packageRoot)), "utf8");

/** The README's TypeScript example: its one `ts` block. */
function example(): string {
  const blocks = [...readme.matchAll(/^```ts\n([\s\S]*?)^```$/gm)].map((match) => match[1]!);
  assert.equal(blocks.length, 1, "the README has one TypeScript example");
  return blocks[0]!;
}

describe("the README", () => {
  it("says the package is unstable, as the manifest does", () => {
    assert.match(readme, /^> \*\*Unstable\.\*\*/m);
    const manifest = JSON.parse(readFileSync(new URL("package.json", pathToFileURL(packageRoot)), "utf8")) as { description: string };
    assert.match(manifest.description, /^Unstable\./);
  });

  it("has an example that type-checks against the package", () => {
    const dir = `${packageRoot}dist/test/readme`;
    mkdirSync(dir, { recursive: true });
    const file = `${dir}/example.ts`;
    writeFileSync(file, example());
    const program = ts.createProgram([file], {
      strict: true,
      exactOptionalPropertyTypes: true,
      noUncheckedIndexedAccess: true,
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      noEmit: true,
      skipLibCheck: true,
      types: [],
      paths: { "@nx-lang/previewer": [`${packageRoot}src/index.ts`] },
    });
    const diagnostics = ts.getPreEmitDiagnostics(program).map((diagnostic) =>
      ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n"),
    );
    assert.deepEqual(diagnostics, []);
  });

  it("has an example that runs as it says", async () => {
    const dir = `${packageRoot}dist/test/readme`;
    mkdirSync(dir, { recursive: true });
    const javascript = ts
      .transpileModule(example(), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } })
      .outputText.replace('"@nx-lang/previewer"', JSON.stringify(pathToFileURL(`${packageRoot}dist/src/index.js`).href));
    const file = `${dir}/example.mjs`;
    writeFileSync(file, javascript);
    const { previewFlow } = (await import(pathToFileURL(file).href)) as {
      previewFlow(images: unknown): unknown;
    };
    assert.deepEqual(previewFlow(images()), { branches: ["designer", "engineer"], completed: true, teamSize: 24 });
  });
});
