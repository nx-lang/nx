import assert from "node:assert/strict";
import { test } from "node:test";

import { linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

import { checkAgentTools, createAgentTools, type AgentHttpRequestCall, type ToolResult } from "../src/execute.js";
import type { NormalizedAgent } from "../src/index.js";
import { normalizeAgent } from "../src/normalize.js";
import { chatToolContext, compileProgram, disposeCompiled } from "./nx-support.js";

const source = `
/// A plan a team can buy.
type Plan = { name:string seats:int monthlyPrice:int }

/// Finds the plans that fit a team, for the conversation that asks.
let findPlans(teamSize: int, context: ChatToolContext): Plan* = {
  <Plan name={"Team for " + context.conversationId} seats={teamSize} monthlyPrice=20 />
}

/// Looks an order up by its identifier.
let lookupOrder(orderId: string): HttpArguments = { <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> /> }

let shop = <HttpConnection name="shop" baseUrl="https://api.example.com/v1/" />

let root(): Agent = {
  <Agent name="support" model="balanced" tools={
    <FunctionTool function={findPlans} />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />
    <RecordSearchTool recordKind="company" />
    <WebSearchTool allowedDomains={ "docs.example.com" } />
  }>Be brief.</Agent>
}
`;

/** What a host stores between the two halves: text, and nothing else. */
interface StoredBundle {
  readonly definition: string;
  readonly images: readonly { readonly identity: string; readonly base64: string }[];
}

/** The compile-time half: where the compiler is. It answers what is stored, as text. */
function compileTime(): StoredBundle {
  const compiled = compileProgram(source);
  const normalized = normalizeAgent(compiled.agent(), {
    schemas: compiled.artifact,
    toolContextType: chatToolContext,
    toolTypes: {
      RecordSearchTool: (value) => ({
        name: "search_records",
        description: "Searches records.",
        inputSchema: { type: "object", properties: { query: { type: "string" } }, required: ["query"], additionalProperties: false },
        annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
        kind: "builtin",
        config: { recordKind: (value as unknown as { recordKind: string }).recordKind },
      }),
    },
  });
  assert.equal(normalized.ok, true, JSON.stringify(normalized.diagnostics, null, 2));
  const { definition } = normalized as { definition: NormalizedAgent };
  // The host's check when it compiles: every function the definition names is in the program.
  assert.deepEqual(checkAgentTools(definition, compiled.program), []);
  return {
    definition: JSON.stringify(definition),
    images: compiled.images.map((image) => ({ identity: image.identity, base64: Buffer.from(image.bytes).toString("base64") })),
  };
}

/**
 * The run-time half: what a host with no compiler does. Everything it is given is text, and what it
 * imports is this package's `execute` entry and the IR runtime.
 */
async function runTime(bundle: StoredBundle): Promise<{ readonly results: Readonly<Record<string, ToolResult>>; readonly requests: readonly AgentHttpRequestCall[]; readonly kinds: readonly string[] }> {
  const prepared = new Map(bundle.images.map((image) => [image.identity, prepareNxIrModule(new Uint8Array(Buffer.from(image.base64, "base64")))]));
  const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });
  const requests: AgentHttpRequestCall[] = [];
  const tools = createAgentTools(JSON.parse(bundle.definition) as NormalizedAgent, program, {
    request: async (call) => {
      requests.push(call);
      return { status: 200, body: { state: "shipped" } };
    },
    executors: { builtin: async (tool, input) => ({ kind: (tool.config as { recordKind: string }).recordKind, query: (input as { query: string }).query }) },
  });
  const context = { callId: "turn_7:1:0", context: { $type: "ChatToolContext", conversationId: "conv_9" } };
  const results: Record<string, ToolResult> = {};
  const inputs: Readonly<Record<string, unknown>> = { find_plans: { teamSize: 5 }, lookup_order: { orderId: "A 1" }, search_records: { query: "acme" } };
  for (const tool of tools) {
    if (tool.execute !== undefined) {
      results[tool.name] = await tool.execute(inputs[tool.name], context);
    }
  }
  return { results, requests, kinds: tools.map((tool) => `${tool.name}:${tool.kind}:${tool.execute === undefined ? "described" : "executable"}`) };
}

test("an agent compiled in one place runs in another, from the stored definition and the emitted IR alone", async () => {
  const bundle = compileTime();
  // Everything the compiler made is released before the run-time half begins. Had that half
  // reached the artifact, the registry or the build context, it would fail on a disposed resource.
  disposeCompiled();
  assert.equal(typeof bundle.definition, "string");
  // The entry module and the two libraries it reaches. Nothing here uses a range, so no prelude
  // image is emitted; the runtime serves its own when a program needs one.
  assert.deepEqual(bundle.images.map((image) => image.identity).sort(), ["@nx/agent/agent.nx", "host/Chat.nx", "main.nx"]);

  const { results, requests, kinds } = await runTime(bundle);
  assert.deepEqual(kinds, [
    "find_plans:function:executable",
    "lookup_order:http:executable",
    "search_records:builtin:executable",
    "web_search:provider:described",
  ]);

  const findPlans = results["find_plans"]!;
  assert.equal(findPlans.ok, true, JSON.stringify(findPlans));
  assert.deepEqual((findPlans as { output: unknown }).output, [{ $type: "Plan", name: "Team for conv_9", seats: 5, monthlyPrice: 20 }]);
  assert.ok(findPlans.usage!.operations! > 0);

  assert.deepEqual(results["lookup_order"], { ok: true, output: { status: 200, body: { state: "shipped" } }, usage: results["lookup_order"]!.usage });
  assert.equal(requests.length, 1);
  assert.deepEqual(requests[0]!.request, { method: "GET", url: "https://api.example.com/v1/orders/A%201", headers: {} });
  assert.equal(requests[0]!.callId, "turn_7:1:0");

  assert.deepEqual(results["search_records"], { ok: true, output: { kind: "company", query: "acme" } });
  assert.equal("web_search" in results, false);
});

test("a released artifact cannot be used, which is what the test above relies on", () => {
  const compiled = compileProgram(source);
  compiled.artifact.functionSchema({ module: "main.nx", name: "lookupOrder" });
  disposeCompiled();
  assert.throws(() => compiled.artifact.functionSchema({ module: "main.nx", name: "lookupOrder" }), /disposed/i);
  assert.throws(() => compiled.artifact.generateNxIr(), /disposed/i);
});
