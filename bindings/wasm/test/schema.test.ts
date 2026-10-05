import { afterAll, describe, expect, it } from "vitest";

import { NxDisposedResourceError, NxEvaluationError } from "../src/errors.js";
import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import { corpus } from "./schema-corpus.js";
import { nxModule } from "./support.js";

const SCHEMA = "https://json-schema.org/draft/2020-12/schema";

const documented = [
  "/// A plan a team can buy.",
  "type Plan = {",
  "  name:string   /// The plan's display name.",
  "}",
  "",
  "/// Finds the plans that fit a team.",
  "///",
  "/// Plans are sorted by price.",
  "let findPlans(",
  "  teamSize:int,   /// Number of people who need a seat.",
  "  maxMonthlyPrice?:int",
  "): Plan* = { <Plan name=\"Team\" /> }",
  "",
  "let render(template:<function Item:string />: string, title:string): string = { title }"
].join("\n");

const toolContext = { module: "@nx/agent/agent.nx", name: "ToolContext" } as const;

const lookupOrder = [
  'import "@nx/agent"',
  "let lookupOrder(orderId:string, context:ToolContext): HttpArguments = { <HttpArguments /> }"
].join("\n");

describe("declaration schemas", () => {
  const host: NxHost = createNxHost(nxModule);

  afterAll(() => {
    host.dispose();
  });

  it("describes a documented function while compiling, and the artifact still emits", () => {
    const artifact = host.buildWorkspaceArtifact({
      modules: [{ identity: "tools.nx", source: documented }],
      entry: "tools.nx"
    });
    try {
      const schema = artifact.functionSchema({ name: "findPlans" });
      expect(schema.module).toBe("tools.nx");
      expect(schema.description).toBe("Finds the plans that fit a team.\n\nPlans are sorted by price.");
      expect(schema.summary).toBe("Finds the plans that fit a team.");
      expect(schema.parameters).toEqual([
        {
          name: "teamSize",
          type: "int",
          required: true,
          description: "Number of people who need a seat."
        },
        { name: "maxMonthlyPrice", type: "int", required: false }
      ]);
      expect(schema.inputSchema).toEqual({
        $schema: SCHEMA,
        type: "object",
        properties: {
          teamSize: { type: "integer", description: "Number of people who need a seat." },
          maxMonthlyPrice: { type: "integer" }
        },
        required: ["teamSize"],
        additionalProperties: false
      });
      expect(schema.outputSchema).toMatchObject({
        type: "array",
        items: { $ref: "#/$defs/Plan" },
        $defs: {
          Plan: {
            description: "A plan a team can buy.",
            properties: { name: { type: "string", description: "The plan's display name." } }
          }
        }
      });
      expect(schema.resultType).toBe("Plan*");
      // The span starts at `let`; the doc comment above it is not part of the declaration.
      expect(schema.declaration).toMatchObject({ startLine: 9, endLine: 12 });
      expect(schema.diagnostics).toEqual([]);

      // The artifact emits the image a runtime calls the function from.
      expect(artifact.generateNxIr().map((image) => image.identity)).toEqual(["tools.nx"]);
    } finally {
      artifact.dispose();
    }
  });

  it("describes a type in either direction", () => {
    const artifact = host.buildProgramArtifact(documented, { fileName: "tools.nx" });
    try {
      const output = artifact.typeSchema({ module: "tools.nx", name: "Plan" });
      expect(output.description).toBe("A plan a team can buy.");
      expect(output.schema).toMatchObject({
        $ref: "#/$defs/Plan",
        $defs: { Plan: { required: ["$type", "name"] } }
      });
      const input = artifact.typeSchema({ name: "Plan" }, { direction: "input" });
      expect(input.schema).toMatchObject({ $defs: { Plan: { required: ["name"] } } });
    } finally {
      artifact.dispose();
    }
  });

  it("describes a library function built against a registry", () => {
    const registry = host.createLibraryRegistry();
    registry.loadLibrary({
      root: "libraries/catalog",
      modules: [
        {
          identity: "Plans.nx",
          source: "export let findPlans(teamSize:int): string* = { \"Team\" }"
        }
      ]
    });
    const context = registry.createBuildContext();
    const artifact = host.buildWorkspaceArtifact({
      modules: [{ identity: "main.nx", source: "let root() = { findPlans(2) }" }],
      entry: "main.nx",
      implicitImports: ["libraries/catalog"],
      buildContext: context
    });
    try {
      const schema = artifact.functionSchema({
        module: "libraries/catalog/Plans.nx",
        name: "findPlans"
      });
      expect(schema.module).toBe("libraries/catalog/Plans.nx");
      expect(schema.outputSchema).toEqual({
        $schema: SCHEMA,
        type: "array",
        items: { type: "string" }
      });
    } finally {
      artifact.dispose();
      context.dispose();
      registry.dispose();
    }
  });

  it("leaves a host-supplied parameter out of the input schema", () => {
    const artifact = host.buildProgramArtifact(lookupOrder, { fileName: "tools.nx" });
    try {
      const schema = artifact.functionSchema({ name: "lookupOrder" }, { hostSuppliedTypes: [toolContext] });
      expect(Object.keys(schema.inputSchema!["properties"] as object)).toEqual(["orderId"]);
      expect(schema.inputSchema!["required"]).toEqual(["orderId"]);
      expect(schema.parameters[1]).toEqual({
        name: "context",
        type: "ToolContext",
        required: true,
        typeRef: toolContext,
        hostSupplied: toolContext
      });
      expect(schema.diagnostics).toEqual([]);

      // Without the option the abstract `ToolContext`, which nothing here extends, has no JSON form.
      const unlisted = artifact.functionSchema({ name: "lookupOrder" });
      expect(unlisted.inputSchema).toBeUndefined();
      expect(unlisted.parameters[1]!.hostSupplied).toBeUndefined();
    } finally {
      artifact.dispose();
    }
  });

  it("names the host-supplied types a parameter's type holds, and keeps the parameter", () => {
    const source = [
      'import "@nx/agent"',
      "type ChatContext extends ToolContext = { conversationId:string }",
      "type Request = { orderId:string context:ChatContext }",
      "let send(contexts:ChatContext+, request:Request, orderId:string, context:ChatContext): string = { orderId }"
    ].join("\n");
    const artifact = host.buildProgramArtifact(source, { fileName: "tools.nx" });
    try {
      const schema = artifact.functionSchema({ name: "send" }, { hostSuppliedTypes: [toolContext] });
      expect(schema.parameters.map((parameter) => [parameter.name, parameter.hostSuppliedWithin])).toEqual([
        ["contexts", [toolContext]],
        ["request", [toolContext]],
        ["orderId", undefined],
        ["context", undefined]
      ]);
      // A parameter that holds none, and one that is host-supplied, have no such member at all.
      expect("hostSuppliedWithin" in schema.parameters[2]!).toBe(false);
      expect("hostSuppliedWithin" in schema.parameters[3]!).toBe(false);
      expect(schema.parameters[3]!.hostSupplied).toEqual(toolContext);
      // Holding one changes nothing else: both parameters are still arguments.
      expect(Object.keys(schema.inputSchema!["properties"] as object)).toEqual(["contexts", "request", "orderId"]);

      // With no types listed there is nothing to hold.
      const unlisted = artifact.functionSchema({ name: "send" });
      expect(unlisted.parameters.every((parameter) => parameter.hostSuppliedWithin === undefined)).toBe(true);
    } finally {
      artifact.dispose();
    }
  });

  it("answers a type with no JSON form as data", () => {
    const artifact = host.buildProgramArtifact(documented, { fileName: "tools.nx" });
    try {
      const schema = artifact.functionSchema({ name: "render" });
      expect(schema.inputSchema).toBeUndefined();
      expect(schema.outputSchema).toEqual({ $schema: SCHEMA, type: "string" });
      expect(schema.diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        "schema-inexpressible-type"
      ]);
      expect(schema.diagnostics[0]!.message).toContain("`template`");
      expect(schema.diagnostics[0]!.labels[0]).toMatchObject({
        file: "tools.nx",
        span: { startLine: 14 },
        primary: true
      });
    } finally {
      artifact.dispose();
    }
  });

  it("throws for an unknown function and stays usable", () => {
    const artifact = host.buildProgramArtifact(documented, { fileName: "tools.nx" });
    try {
      let thrown: unknown;
      try {
        artifact.functionSchema({ name: "missing" });
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(NxEvaluationError);
      expect((thrown as NxEvaluationError).diagnostics[0]!.code).toBe("schema-unknown-declaration");
      expect(() => artifact.typeSchema({ name: "Missing" })).toThrowError(NxEvaluationError);
      expect(artifact.functionSchema({ name: "findPlans" }).name).toBe("findPlans");
    } finally {
      artifact.dispose();
    }
  });

  it("keeps the schemas after the artifact is disposed, and refuses a disposed artifact", () => {
    const artifact = host.buildProgramArtifact(documented, { fileName: "tools.nx" });
    const schema = artifact.functionSchema({ name: "findPlans" });
    artifact.dispose();
    expect(JSON.parse(JSON.stringify(schema.inputSchema))).toEqual(schema.inputSchema);
    expect(() => artifact.functionSchema({ name: "findPlans" })).toThrowError(
      NxDisposedResourceError
    );
    expect(() => artifact.typeSchema({ name: "Plan" })).toThrowError(NxDisposedResourceError);
  });

  it("answers every corpus function with the Rust golden documents", () => {
    for (const entry of corpus) {
      const artifact = host.buildWorkspaceArtifact({
        modules: [{ identity: entry.identity, source: entry.source }],
        entry: entry.identity
      });
      try {
        for (const [name, golden] of Object.entries(entry.golden)) {
          const schema = artifact.functionSchema({ name });
          // Text equality checks key order as well as content.
          expect(JSON.stringify(schema.inputSchema), `${entry.identity} ${name}`).toBe(
            JSON.stringify(golden.inputSchema)
          );
          expect(JSON.stringify(schema.outputSchema), `${entry.identity} ${name}`).toBe(
            JSON.stringify(golden.outputSchema)
          );
          expect(schema.diagnostics.map((diagnostic) => diagnostic.code)).toEqual(
            golden.diagnostics
          );
        }
      } finally {
        artifact.dispose();
      }
    }
  });
});
