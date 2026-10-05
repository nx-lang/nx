import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { packageRoot } from "./support.js";

// A consumer of the built package, compiled under each way TypeScript resolves an import. The
// probe compiles only when each import is the real type: under `any`, or the error type an import
// that did not resolve becomes, an `@ts-expect-error` line has no error to expect, which is one.
const probe = `
import type { Agent, FunctionTool, HttpMethod, NormalizedAgent, NxFunctionRef, Tool, ToolContext } from "@nx-lang/agent";
import type { AgentSchemaSource } from "@nx-lang/agent/normalize";
import type { AgentTool, ToolCallContext } from "@nx-lang/agent/execute";
import type { ToAiSdkToolsOptions } from "@nx-lang/agent/ai-sdk";

const discriminator: Agent["$type"] = "Agent";
// @ts-expect-error a wrong discriminator is refused
const wrongDiscriminator: Agent["$type"] = "Other";
const reference: NxFunctionRef["$type"] = "Function";
// @ts-expect-error
const wrongReference: NxFunctionRef["$type"] = "Other";
const method: HttpMethod = "get";
// @ts-expect-error
const wrongMethod: HttpMethod = "fetch";
// @ts-expect-error a FunctionTool needs its function
const tool: FunctionTool = { $type: "FunctionTool" };
// @ts-expect-error a ToolContext needs its callId
const context: ToolContext = { $type: "ChatToolContext" };
const base: Tool = { $type: "RecordSearchTool", name: "search_records" };
// @ts-expect-error the format version is the literal 1
const version: NormalizedAgent["formatVersion"] = 2;
// @ts-expect-error a schema source answers functionSchema
const source: AgentSchemaSource = {};
// @ts-expect-error a tool has a name
const executable: AgentTool = {};

// A host's context type as \`nxlang typegen\` writes it: an interface, which has no index signature.
interface ChatToolContext extends ToolContext {
  $type: "ChatToolContext";
  conversationId: string;
}
declare const generated: ChatToolContext;
// The package sets \`callId\`, so a host need not make one up to build its record.
declare const withoutCallId: Omit<ChatToolContext, "callId">;
const fromGenerated: ToolCallContext = { callId: "turn_7:1:0", context: generated };
const fromPartial: ToolCallContext = { callId: "turn_7:1:0", context: withoutCallId };
const fromLiteral: ToolCallContext = { callId: "turn_7:1:0", context: { $type: "ChatToolContext", conversationId: "conv_9" } };
// @ts-expect-error a context record has its $type
const untyped: ToolCallContext = { callId: "turn_7:1:0", context: { conversationId: "conv_9" } };
// @ts-expect-error a context record is a record
const notARecord: ToolCallContext = { callId: "turn_7:1:0", context: "conv_9" };
const sdkGenerated: ToAiSdkToolsOptions = { context: () => generated };
const sdkPartial: ToAiSdkToolsOptions = { context: () => withoutCallId };
const sdkLiteral: ToAiSdkToolsOptions = { context: () => ({ $type: "ChatToolContext", conversationId: "conv_9" }) };
export { base, context, discriminator, executable, method, reference, source, tool, version, wrongDiscriminator, wrongMethod, wrongReference };
export { fromGenerated, fromLiteral, fromPartial, notARecord, sdkGenerated, sdkLiteral, sdkPartial, untyped };
`;

for (const [module, moduleResolution] of [
  ["nodenext", "nodenext"],
  ["esnext", "bundler"],
] as const) {
  test(`the package's types resolve for a consumer under moduleResolution ${moduleResolution}`, () => {
    const consumer = mkdtempSync(join(tmpdir(), "nx-agent-consumer-"));
    try {
      mkdirSync(join(consumer, "node_modules", "@nx-lang"), { recursive: true });
      symlinkSync(packageRoot, join(consumer, "node_modules", "@nx-lang", "agent"), "dir");
      writeFileSync(join(consumer, "package.json"), JSON.stringify({ name: "consumer", private: true, type: "module" }));
      writeFileSync(join(consumer, "probe.ts"), probe);
      const tsc = join(packageRoot, "node_modules", ".bin", "tsc");
      const result = spawnSync(
        tsc,
        ["--noEmit", "--strict", "--skipLibCheck", "--target", "es2022", "--module", module, "--moduleResolution", moduleResolution, "probe.ts"],
        { cwd: consumer, encoding: "utf8" },
      );
      assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
    } finally {
      rmSync(consumer, { recursive: true, force: true });
    }
  });
}
