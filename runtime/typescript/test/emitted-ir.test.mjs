import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  evaluateComponent,
  evaluateFunction,
  initializeComponent,
  prepareNxIrProgram,
} from "../dist/src/index.js";

const testRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = resolve(testRoot, "../../..");

function assertEqual(actual, expected) {
  const actualJson = stableJson(actual);
  const expectedJson = stableJson(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
  }
}

function stableJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }
  return result.stdout;
}

function runNxCli(args) {
  return run("cargo", ["run", "-q", "-p", "nx-cli", "--", ...args]);
}

function withSource(source, runTest) {
  const dir = mkdtempSync(join(tmpdir(), "nx-ir-runtime-"));
  try {
    const sourcePath = join(dir, "test.nx");
    writeFileSync(sourcePath, source);
    runTest(dir, sourcePath);
  } finally {
    rmSync(dir, { force: true, recursive: true });
  }
}

function emitIr(dir, sourcePath) {
  const outputPath = join(dir, "ir");
  runNxCli(["codegen", sourcePath, "--target", "nx-ir", "--output", outputPath]);
  return JSON.parse(readFileSync(join(outputPath, "test.nxir.json"), "utf8"));
}

function nativeJson(sourcePath) {
  return JSON.parse(runNxCli(["run", sourcePath, "--format", "json"]));
}

function nativeFailure(sourcePath) {
  return spawnSync(
    "cargo",
    ["run", "-q", "-p", "nx-cli", "--", "run", sourcePath, "--format", "json"],
    {
      cwd: repoRoot,
      encoding: "utf8",
    },
  );
}

function generatedJsJson(outputPath, script) {
  writeFileSync(join(outputPath, "package.json"), "{ \"type\": \"module\" }\n");
  return JSON.parse(run("node", ["--input-type=module", "--eval", script], { cwd: outputPath }));
}

withSource(
  `
let answer(): int = { 41 }
let root(): int = { answer() + 1 + (7 / 2) + (7 % 2) }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - emitted function IR matches native interpreter output");
  },
);

withSource(
  `
let root(): int = { 1 / 0 }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    let runtimeFailed = false;
    try {
      evaluateFunction(prepared, "root");
    } catch (error) {
      runtimeFailed = String(error).includes("Division by zero");
    }
    if (!runtimeFailed) {
      throw new Error("Expected TypeScript IR runtime to fail with Division by zero");
    }

    const native = nativeFailure(sourcePath);
    if (native.status === 0 || !`${native.stdout}\n${native.stderr}`.includes("Division by zero")) {
      throw new Error(
        `Expected native interpreter to fail with Division by zero\nstdout:\n${native.stdout}\nstderr:\n${native.stderr}`,
      );
    }
    console.log("ok - emitted division-by-zero IR matches native interpreter failure");
  },
);

withSource(
  `
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root, SearchBoxSchema } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify({
  descriptor: root(),
  init: SearchBoxSchema.initializeJson({}),
  evaluated: SearchBoxSchema.evaluateJson({}, { query: "docs" })
}));
`,
    );

    assertEqual(
      {
        descriptor: evaluateFunction(prepared, "root"),
        init: initializeComponent(prepared, "SearchBox"),
        evaluated: evaluateComponent(prepared, "SearchBox", {}, { query: "docs" }).rendered,
      },
      generated,
    );
    console.log("ok - emitted component IR matches generated JavaScript behavior");
  },
);
