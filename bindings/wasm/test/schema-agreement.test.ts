import {
  callFunction,
  linkNxIrProgram,
  NxIrRuntimeError,
  prepareNxIrModule,
  type NxCanonicalValue,
  type NxPreparedModule,
  type NxPreparedProgram
} from "@nx-lang/ir-runtime";
import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";
import { afterAll, describe, expect, it } from "vitest";

import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import type { NxFunctionSchema, NxJsonSchema, NxJsonSchemaValue } from "../src/types.js";
import { corpus, type CorpusModule } from "./schema-corpus.js";
import { nxModule } from "./support.js";

/**
 * The export's contract with the runtimes' boundary, checked against the TypeScript IR runtime:
 * every value valid against a function's input schema is accepted as its arguments, and every
 * value a call returns is valid against its output schema.
 */
describe("declaration schemas agree with the IR runtime's boundary", () => {
  const host: NxHost = createNxHost(nxModule);
  const ajv = new Ajv2020({ allErrors: true, strict: true });

  afterAll(() => {
    host.dispose();
  });

  for (const entry of corpus) {
    it(`agrees for every function of ${entry.identity}`, () => {
      const { program, schemas } = build(host, entry);
      for (const [name, schema] of schemas) {
        const input = schema.inputSchema === undefined ? undefined : ajv.compile(schema.inputSchema);
        const output = schema.outputSchema === undefined ? undefined : ajv.compile(schema.outputSchema);
        if (input === undefined) {
          // A parameter with no JSON form, such as a function value, comes from a host rather than
          // from JSON, so nothing is generated; a hand-written case still checks the result.
          for (const [index, argumentCase] of (entry.args[name] ?? []).entries()) {
            const where = `${entry.identity} ${name} (case ${index})`;
            const result = call(program, entry.identity, name, argumentCase.args, where);
            if ("result" in argumentCase) {
              expect(result, where).toEqual(argumentCase.result);
            }
            if (output !== undefined) {
              expectValid(output, result, `${where} result`);
            }
          }
          continue;
        }

        const cases: { label: string; args: Record<string, unknown>; result?: unknown }[] = [
          ...(entry.args[name] ?? []).map((argumentCase, index) => ({
            label: `case ${index}`,
            ...argumentCase
          })),
          ...generatedArguments(schema.inputSchema!).map((args, index) => ({
            label: `generated ${index}`,
            args
          }))
        ];
        expect(cases.length, `${entry.identity} ${name} has arguments to call it with`).toBeGreaterThan(0);

        for (const argumentCase of cases) {
          const where = `${entry.identity} ${name} (${argumentCase.label}: ${JSON.stringify(argumentCase.args)})`;
          expectValid(input, argumentCase.args, `${where} arguments`);
          const result = call(program, entry.identity, name, argumentCase.args, where);
          if ("result" in argumentCase) {
            expect(result, where).toEqual(argumentCase.result);
          }
          if (output !== undefined) {
            expectValid(output, result, `${where} result`);
          }
        }
      }
    });
  }

  it("covers the result shapes the canonical encoding has", () => {
    // Each of these is pinned by an argument case above, so a corpus change that drops one fails
    // here rather than silently narrowing what the agreement covers.
    const results = corpus.flatMap((entry) =>
      Object.values(entry.args).flatMap((cases) => cases.map((argumentCase) => argumentCase.result))
    );
    const covered = (predicate: (value: unknown) => boolean) => results.some(predicate);
    const record = (value: unknown): value is Record<string, unknown> =>
      typeof value === "object" && value !== null && !Array.isArray(value);

    expect(covered((value) => value === null), "a null T? result").toBe(true);
    expect(covered((value) => Array.isArray(value) && value.length === 0), "an empty T*").toBe(true);
    expect(covered((value) => record(value) && value["$type"] === "Box" && !("tags" in value)), "an omitted optional field").toBe(true);
    expect(covered((value) => record(value) && value["durationMinutes"] === 30), "a defaulted field").toBe(true);
    expect(covered((value) => value === "idle"), "a constant case").toBe(true);
    expect(covered((value) => record(value) && value["$type"] === "LoadState.failed"), "a payload case").toBe(true);
    expect(covered((value) => record(value) && value["$type"] === "Circle"), "a descendant at an abstract site").toBe(true);
    expect(covered((value) => record(value) && value["$type"] === "Page"), "an applied generic record").toBe(true);
  });

  it("accepts a subtype declared in a module nothing references, as the schema lists it", () => {
    const modules = [
      { identity: "main.nx", source: 'import "./base.nx"\nimport "./x.nx"\nlet f(s:Base): string = { "ok" }' },
      { identity: "base.nx", source: "export abstract type Base = { id:int }\nexport type A extends Base = { a:string }" },
      { identity: "x.nx", source: 'import "./base.nx"\nexport type X extends Base = { x:string }' },
      { identity: "y.nx", source: 'import "./base.nx"\nexport type Y extends Base = { y:string }' }
    ];
    const artifact = host.buildWorkspaceArtifact({ modules, entry: "main.nx" });
    try {
      const schema = artifact.functionSchema({ name: "f" });
      const input = ajv.compile(schema.inputSchema!);
      const prepared = new Map(
        artifact.generateNxIr({ modules: [] }).map((image) => [image.identity, prepareNxIrModule(image.bytes)])
      );
      const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });
      for (const args of [
        { s: { $type: "A", id: 1, a: "q" } },
        { s: { $type: "X", id: 1, x: "q" } },
        { s: { $type: "Y", id: 1, y: "q" } }
      ]) {
        expectValid(input, args, `f ${JSON.stringify(args)}`);
        expect(call(program, "main.nx", "f", args, `f ${JSON.stringify(args)}`)).toBe("ok");
      }
    } finally {
      artifact.dispose();
    }
  });

  it("accepts an argument record with no $type and normalizes it to the declared record", () => {
    const entry = corpus.find((candidate) => candidate.identity === "records.nx")!;
    const { program, schemas } = build(host, entry);
    const input = ajv.compile(schemas.get("book")!.inputSchema!);
    const args = { request: { host: "Ada" } };
    expectValid(input, args, "book arguments");
    expect(call(program, entry.identity, "book", args, "book")).toEqual({
      $type: "Booking",
      host: "Ada",
      durationMinutes: 30
    });
  });
});

/** Builds a corpus module, emits every image it needs, links them, and describes its functions. */
function build(
  host: NxHost,
  entry: CorpusModule
): { program: NxPreparedProgram; schemas: Map<string, NxFunctionSchema> } {
  const artifact = host.buildWorkspaceArtifact({
    modules: [{ identity: entry.identity, source: entry.source }],
    entry: entry.identity
  });
  try {
    const prepared = new Map<string, NxPreparedModule>(
      artifact.generateNxIr({ modules: [] }).map((image) => [image.identity, prepareNxIrModule(image.bytes)])
    );
    const program = linkNxIrProgram(prepared.get(entry.identity)!, {
      resolve: (identity) => prepared.get(identity)
    });
    const schemas = new Map(
      Object.keys(entry.golden).map((name) => [name, artifact.functionSchema({ name })] as const)
    );
    return { program, schemas };
  } finally {
    artifact.dispose();
  }
}

function call(
  program: NxPreparedProgram,
  module: string,
  name: string,
  args: Record<string, unknown>,
  where: string
): NxCanonicalValue {
  try {
    return callFunction(
      program,
      { $type: "Function", module, name },
      args as Record<string, NxCanonicalValue>
    );
  } catch (error) {
    if (error instanceof NxIrRuntimeError) {
      const codes = error.diagnostics.map((diagnostic) => diagnostic.code).join(", ");
      throw new Error(`${where} was refused (${codes}): ${error.message}`);
    }
    throw error;
  }
}

function expectValid(validate: ValidateFunction, value: unknown, where: string): void {
  const valid = validate(value);
  expect(valid, `${where}: ${JSON.stringify(validate.errors)}`).toBe(true);
}

/**
 * Arguments generated from an input schema alone: every property, then only the required ones, once
 * for each branch of the widest `anyOf` they meet. A schema wider than the runtime's boundary yields
 * arguments the runtime refuses, which is what makes this check catch a widened mapping.
 */
function generatedArguments(schema: NxJsonSchema): Record<string, unknown>[] {
  const defs = (schema["$defs"] ?? {}) as Record<string, NxJsonSchema>;
  const generated: Record<string, unknown>[] = [];
  const width = Math.max(1, widestAnyOf(schema, defs, new Set()));
  for (const all of [true, false]) {
    for (let branch = 0; branch < width; branch++) {
      generated.push(sample(schema, defs, { all, branch, depth: 0 }) as Record<string, unknown>);
    }
  }
  return generated;
}

interface SampleOptions {
  readonly all: boolean;
  readonly branch: number;
  readonly depth: number;
}

/** A value valid against `schema`: the first `enum` value, one array item, the chosen branch. */
function sample(schema: NxJsonSchemaValue, defs: Record<string, NxJsonSchema>, options: SampleOptions): unknown {
  if (typeof schema !== "object" || schema === null || Array.isArray(schema)) {
    return "any";
  }
  const node = schema as NxJsonSchema;
  if (Object.keys(node).every((key) => key === "description" || key === "default")) {
    // A schema that admits anything admits `null` too, which is the value a runtime is most likely
    // to refuse; a mapping widened to `{}` is caught here.
    return null;
  }
  if (typeof node["$ref"] === "string") {
    return sample(defs[node["$ref"].replace("#/$defs/", "")]!, defs, options);
  }
  if ("const" in node) {
    return node["const"];
  }
  if (Array.isArray(node["enum"])) {
    const values = node["enum"] as readonly NxJsonSchemaValue[];
    return values[options.branch % values.length];
  }
  if (Array.isArray(node["anyOf"])) {
    const branches = node["anyOf"] as readonly NxJsonSchemaValue[];
    return sample(branches[options.branch % branches.length]!, defs, options);
  }
  switch (node["type"]) {
    case "string":
      return "text";
    case "boolean":
      return true;
    case "integer":
      return 1;
    case "number":
      return 1.5;
    case "null":
      return null;
    case "array":
      return [sample(node["items"] ?? {}, defs, options)];
    case "object": {
      const properties = (node["properties"] ?? {}) as Record<string, NxJsonSchemaValue>;
      const required = new Set((node["required"] ?? []) as readonly string[]);
      // A recursive type stops at its optional fields.
      const all = options.all && options.depth < 3;
      const value: Record<string, unknown> = {};
      for (const [key, property] of Object.entries(properties)) {
        if (required.has(key) || all) {
          value[key] = sample(property, defs, { ...options, depth: options.depth + 1 });
        }
      }
      return value;
    }
    default:
      // `{}`: the top type admits anything.
      return { any: [1] };
  }
}

function widestAnyOf(schema: NxJsonSchemaValue, defs: Record<string, NxJsonSchema>, seen: Set<string>): number {
  if (typeof schema !== "object" || schema === null) {
    return 0;
  }
  if (Array.isArray(schema)) {
    return Math.max(0, ...schema.map((item) => widestAnyOf(item, defs, seen)));
  }
  const node = schema as NxJsonSchema;
  let widest = 0;
  if (typeof node["$ref"] === "string") {
    const key = node["$ref"].replace("#/$defs/", "");
    if (!seen.has(key)) {
      seen.add(key);
      widest = widestAnyOf(defs[key]!, defs, seen);
    }
  }
  if (Array.isArray(node["anyOf"])) {
    widest = Math.max(widest, node["anyOf"].length);
  }
  if (Array.isArray(node["enum"])) {
    widest = Math.max(widest, node["enum"].length);
  }
  for (const [key, value] of Object.entries(node)) {
    if (key !== "$defs" && key !== "$ref") {
      widest = Math.max(widest, widestAnyOf(value, defs, seen));
    }
  }
  return widest;
}
