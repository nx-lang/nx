import assert from "node:assert/strict";
import { after, test } from "node:test";

import { measureInputSize } from "@nx-lang/ir-runtime";

import {
  createAgentTools,
  evaluateHttpArguments,
  type AgentHttpRequestCall,
  type AgentHttpRequestFunction,
  type AgentHttpResponse,
  type CreateAgentToolsOptions,
  type EvaluateHttpArgumentsResult,
  type ToolCallContext,
  type ToolResult,
} from "../src/execute.js";
import { NxAgentToolError, buildHttpRequest, type NormalizedAgent, type NormalizedTool } from "../src/index.js";
import { normalizeAgent } from "../src/normalize.js";
import { chatToolContext, compileProgram, disposeCompiled } from "./nx-support.js";

after(disposeCompiled);

const compiled = compileProgram(`
/// A new order, sent as a request body.
type NewOrder = { sku:string quantity:int }

/// Looks an order up by its identifier.
let lookupOrder(orderId: string): HttpArguments = { <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> /> }

/// Looks an order up for the conversation.
let lookupMine(orderId: string, context: ChatToolContext, note?: string): HttpArguments = {
  <HttpArguments
    pathParams=<HttpParam name="orderId" value={orderId} />
    query={ <HttpParam name="conversation" value={context.conversationId} /> <HttpParam name="call" value={context.callId} /> } />
}

/// Places an order for a product.
let placeOrder(sku: string, quantity: int = 1): HttpArguments = {
  <HttpArguments query=<HttpParam name="source" value="agent" /> body=<NewOrder sku={sku} quantity={quantity} /> />
}

/// Lists orders, by a number of pages it works out the slow way.
let listOrders(pages: int): HttpArguments = {
  <HttpArguments query={ for i in 0..pages { for j in 0..pages { <HttpParam name="p" value="1" /> } } } />
}

/// Looks up the order at a position.
let orderAt(position: int): HttpArguments = { <HttpArguments query=<HttpParam name="n" value={if 10 / position > 1 { "many" } else { "few" }} /> /> }

/// Returns its argument, which is not the arguments of a request.
let notArguments(orderId: string): string = { orderId }

let shop = <HttpConnection name="shop" baseUrl="https://api.example.com" />

let root(): Agent = {
  <Agent name="support" tools={
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupMine} />
    <HttpTool connection={shop} method={HttpMethod.post} path="/orders" arguments={placeOrder} />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders" arguments={listOrders} />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders" arguments={orderAt} />
    <HttpTool name="undeclared_parameter" connection={shop} method={HttpMethod.get} path="/orders" arguments={lookupOrder} />
    <HttpTool name="body_on_get" connection={shop} method={HttpMethod.get} path="/orders" arguments={placeOrder} />
  }>Be brief.</Agent>
}
`);

const normalized = normalizeAgent(compiled.agent(), { schemas: compiled.artifact, toolContextType: chatToolContext });
assert.equal(normalized.ok, true, JSON.stringify(normalized.diagnostics, null, 2));
const definition = JSON.parse(JSON.stringify((normalized as { definition: NormalizedAgent }).definition)) as NormalizedAgent;

function definitionOf(name: string): NormalizedTool {
  return definition.tools.find((tool) => tool.name === name)!;
}

// A stored definition whose arguments function, in the linked program, returns a string: one that
// did not come from a checked program, or that is run against a program that has since changed.
const staleTool = { ...definitionOf("lookup_order"), name: "stale", config: { ...definitionOf("lookup_order").config, arguments: { module: "main.nx", name: "notArguments" } } } as NormalizedTool;

/** A request function that records its calls and answers with `respond`. */
function recorder(respond: (call: AgentHttpRequestCall) => AgentHttpResponse | Promise<AgentHttpResponse> = () => ({ status: 200, body: { state: "shipped" } })) {
  const calls: AgentHttpRequestCall[] = [];
  const request: AgentHttpRequestFunction = async (call) => {
    calls.push(call);
    return respond(call);
  };
  return { calls, request };
}

function toolsWith(request: AgentHttpRequestFunction, options: CreateAgentToolsOptions = {}) {
  return new Map(createAgentTools([...definition.tools, staleTool], compiled.program, { request, ...options }).map((tool) => [tool.name, tool]));
}

const chat = { $type: "ChatToolContext", conversationId: "conv_9" };

function failed<T extends { ok: boolean }>(result: T): Extract<T, { ok: false }> {
  assert.equal(result.ok, false, JSON.stringify(result));
  return result as Extract<T, { ok: false }>;
}

function succeeded<T extends { ok: boolean }>(result: T): Extract<T, { ok: true }> {
  assert.equal(result.ok, true, JSON.stringify(result));
  return result as Extract<T, { ok: true }>;
}

async function run(name: string, input: unknown, request: AgentHttpRequestFunction, context: Partial<ToolCallContext> = {}, options: CreateAgentToolsOptions = {}): Promise<ToolResult> {
  return toolsWith(request, options).get(name)!.execute!(input, { callId: "t1:0:0", ...context } as ToolCallContext);
}

function evaluate(name: string | NormalizedTool, input: unknown, context: Partial<ToolCallContext> = {}, options = {}): EvaluateHttpArgumentsResult {
  const tool = typeof name === "string" ? definitionOf(name) : name;
  return evaluateHttpArguments(tool, compiled.program, input, { callId: "t1:0:0", ...context } as ToolCallContext, options);
}

// ---- execute ------------------------------------------------------------------------------------

test("the host's function makes the call, and what it answers is the output", async () => {
  const { calls, request } = recorder();
  const signal = new AbortController().signal;
  const result = succeeded(await run("lookup_order", { orderId: "A1" }, request, { callId: "t7:1:0", signal }));
  assert.deepEqual(result.output, { status: 200, body: { state: "shipped" } });
  assert.equal(calls.length, 1);
  const call = calls[0]!;
  assert.deepEqual(call.request, { method: "GET", url: "https://api.example.com/orders/A1", headers: {} });
  assert.deepEqual(call.arguments, { pathParams: [{ name: "orderId", value: "A1" }], query: [] });
  assert.equal(call.callId, "t7:1:0");
  assert.equal(call.toolName, "lookup_order");
  assert.equal(call.connection, "shop");
  assert.deepEqual(call.annotations, { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: true });
  assert.equal(call.signal, signal);
  assert.deepEqual(Object.keys(call), ["request", "arguments", "callId", "toolName", "connection", "annotations", "signal"]);
});

test("with no signal the request function is handed none", async () => {
  const { calls, request } = recorder();
  await run("lookup_order", { orderId: "A1" }, request);
  assert.equal("signal" in calls[0]!, false);
});

test("a body is sent as JSON, and the request function is handed it as evaluated", async () => {
  const { calls, request } = recorder(() => ({ status: 201, body: { id: "o_1" } }));
  const result = succeeded(await run("place_order", { sku: "X", quantity: 2 }, request));
  assert.deepEqual(result.output, { status: 201, body: { id: "o_1" } });
  const call = calls[0]!;
  assert.deepEqual(call.request, {
    method: "POST",
    url: "https://api.example.com/orders?source=agent",
    headers: { "content-type": "application/json" },
    body: '{"sku":"X","quantity":2}',
  });
  // The structured arguments keep the body's `$type`; the request's body does not have it.
  assert.deepEqual(call.arguments, { pathParams: [], query: [{ name: "source", value: "agent" }], body: { $type: "NewOrder", sku: "X", quantity: 2 } });
  assert.equal(call.annotations.readOnlyHint, false);
});

test("the package sets no idempotency header, and hands over the callId so the host can", async () => {
  const { calls, request } = recorder();
  await run("place_order", { sku: "X" }, request, { callId: "t7:2:0" });
  const headers = Object.keys(calls[0]!.request.headers).map((header) => header.toLowerCase());
  assert.deepEqual(headers, ["content-type"]);
  assert.equal(headers.includes("idempotency-key"), false);
  assert.equal(calls[0]!.callId, "t7:2:0");
  // A parameter's default is the function's to fill.
  assert.equal(calls[0]!.request.body, '{"sku":"X","quantity":1}');
});

test("the arguments function is given the host's context, as a function tool's function is", async () => {
  const { calls, request } = recorder();
  succeeded(await run("lookup_mine", { orderId: "A1", context: { conversationId: "forged" } }, request, { callId: "t7:3:0", context: chat }));
  assert.equal(calls[0]!.request.url, "https://api.example.com/orders/A1?conversation=conv_9&call=t7%3A3%3A0");
  const missing = failed(await run("lookup_mine", { orderId: "A1" }, request));
  assert.equal(missing.error.code, "invalid-context");
  assert.equal(calls.length, 1);
});

test("a non-2xx status is an output, not a failure", async () => {
  for (const response of [{ status: 404, body: "not found" }, { status: 500, body: null }, { status: 302, body: [] }]) {
    const { request } = recorder(() => response);
    assert.deepEqual(succeeded(await run("lookup_order", { orderId: "A1" }, request)).output, response);
  }
});

test("extra result properties are kept", async () => {
  const { request } = recorder(() => ({ status: 200, body: "…", bodyNote: "truncated", bytes: 4096 }));
  assert.deepEqual(succeeded(await run("lookup_order", { orderId: "A1" }, request)).output, { status: 200, body: "…", bodyNote: "truncated", bytes: 4096 });
});

test("a result carries the usage of the arguments function's call", async () => {
  const { request } = recorder();
  const result = succeeded(await run("lookup_order", { orderId: "A1" }, request));
  assert.ok(result.usage!.operations! > 0);
  assert.equal(result.usage!.inputSize, measureInputSize({ $type: "Function", module: "main.nx", name: "lookupOrder" }) + measureInputSize({ orderId: "A1" }));
  assert.deepEqual(result.usage, succeeded(evaluate("lookup_order", { orderId: "A1" })).usage);
});

test("a structural failure never reaches the network", async () => {
  const { calls, request } = recorder();
  const undeclared = failed(await run("undeclared_parameter", { orderId: "A1" }, request));
  assert.equal(undeclared.error.code, "invalid-request");
  assert.match(undeclared.error.message, /Tool 'undeclared_parameter': the path '\/orders' has no placeholder for the path parameter 'orderId'/);
  // The arguments function ran, so the failure that follows it carries what that call used.
  assert.ok(undeclared.usage!.operations! > 0);

  const body = failed(await run("body_on_get", { sku: "X" }, request));
  assert.equal(body.error.code, "invalid-request");
  assert.match(body.error.message, /a GET request carries none/);

  const traversal = failed(await run("lookup_order", { orderId: ".." }, request));
  assert.equal(traversal.error.code, "invalid-request");
  assert.match(traversal.error.message, /has the segment '\.\.'/);
  assert.equal(calls.length, 0);
});

test("a function that does not return HttpArguments is refused at the call, naming the function", async () => {
  const { calls, request } = recorder();
  const result = failed(await run("stale", { orderId: "A1" }, request));
  assert.equal(result.error.code, "invalid-request");
  assert.equal(result.error.message, `Tool 'stale': its arguments function 'notArguments' of module 'main.nx' returned the string "A1", not an 'HttpArguments'.`);
  assert.ok(result.usage!.operations! > 0);
  assert.equal(calls.length, 0);

  const evaluated = failed(evaluate(staleTool, { orderId: "A1" }));
  assert.equal(evaluated.error.code, "invalid-request");
  assert.match(evaluated.error.message, /'notArguments' of module 'main\.nx' returned the string "A1"/);
  assert.ok(evaluated.usage!.operations! > 0);
});

test("the arguments function fails with a function tool's codes, without a request", async () => {
  const { calls, request } = recorder();
  const wrongType = failed(await run("lookup_order", { orderId: 7 }, request));
  assert.equal(wrongType.error.code, "invalid-input");
  assert.equal(wrongType.error.diagnostics![0]!.code, "nx-ir-boundary-type");
  assert.equal(typeof wrongType.usage!.operations, "number");

  assert.equal(failed(await run("lookup_order", {}, request)).error.code, "invalid-input");
  assert.equal(failed(await run("lookup_order", "A1", request)).error.code, "invalid-input");

  const evaluation = failed(await run("order_at", { position: 0 }, request));
  assert.equal(evaluation.error.code, "evaluation-failed");
  assert.equal(evaluation.error.diagnostics![0]!.code, "nx-ir-division-by-zero");

  const budget = failed(await run("list_orders", { pages: 1000 }, request));
  assert.equal(budget.error.code, "resource-limit");
  assert.deepEqual(budget.error.limit, { name: "maxOperations", value: 100_000 });
  assert.ok(budget.usage!.operations! <= 100_000);

  const hostBudget = failed(await run("list_orders", { pages: 1000 }, request, {}, { runtime: { maxOperations: 200_000 } }));
  assert.deepEqual(hostBudget.error.limit, { name: "maxOperations", value: 200_000 });

  assert.equal(failed(await run("lookup_order", { orderId: "A1" }, request, { signal: AbortSignal.abort() })).error.code, "aborted");
  assert.equal(failed(await run("lookup_order", { orderId: "A1" }, request, { callId: "" })).error.code, "invalid-context");
  assert.equal(calls.length, 0);
});

test("the arguments function runs under the same two input limits", async () => {
  const { calls, request } = recorder();
  const large = { orderId: "A1", note: "x".repeat(64 * 999) };
  assert.equal(measureInputSize(large), 1_002);
  const overCap = failed(await run("lookup_mine", large, request, { context: chat }));
  assert.equal(overCap.error.code, "invalid-input");
  assert.deepEqual(overCap.error.limit, { name: "maxArgumentsSize", value: 1_000 });
  assert.equal("usage" in overCap, false);

  const bigContext = { $type: "ChatToolContext", conversationId: "c".repeat(64 * 998) };
  assert.equal(measureInputSize({ ...bigContext, callId: "t1:0:0" }), 1_001);
  const overAllowance = failed(await run("lookup_mine", { orderId: "A1" }, request, { context: bigContext }));
  assert.equal(overAllowance.error.code, "resource-limit");
  assert.deepEqual(overAllowance.error.limit, { name: "maxContextSize", value: 1_000 });
  assert.equal("usage" in overAllowance, false);
  assert.equal(calls.length, 0);

  // The same through the exported evaluation, and the limits are the host's to set there too.
  const evaluatedCap = failed(evaluate("lookup_mine", large, { context: chat }));
  assert.deepEqual([evaluatedCap.error.code, evaluatedCap.error.limit, "usage" in evaluatedCap], ["invalid-input", { name: "maxArgumentsSize", value: 1_000 }, false]);
  const evaluatedAllowance = failed(evaluate("lookup_mine", { orderId: "A1" }, { context: bigContext }));
  assert.deepEqual([evaluatedAllowance.error.code, evaluatedAllowance.error.limit, "usage" in evaluatedAllowance], ["resource-limit", { name: "maxContextSize", value: 1_000 }, false]);
  succeeded(evaluate("lookup_mine", large, { context: bigContext }, { maxArgumentsSize: 2_000, maxContextSize: 2_000 }));
  succeeded(await run("lookup_mine", large, request, { context: bigContext }, { maxArgumentsSize: 2_000, maxContextSize: 2_000 }));
});

test("an error the request function throws fails the call as request-failed, with the usage of the arguments function's call", async () => {
  const { request } = recorder(() => {
    throw new Error("connect ETIMEDOUT");
  });
  const result = failed(await run("lookup_order", { orderId: "A1" }, request));
  assert.equal(result.error.code, "request-failed");
  assert.equal(result.error.message, "connect ETIMEDOUT");
  assert.ok(result.usage!.operations! > 0);
  await assert.doesNotReject(run("lookup_order", { orderId: "A1" }, request));

  const rejecting: AgentHttpRequestFunction = () => Promise.reject("refused");
  assert.deepEqual(failed(await run("lookup_order", { orderId: "A1" }, rejecting)).error, { code: "request-failed", message: "refused" });
});

test("a host policy error keeps its code", async () => {
  const { request } = recorder(() => {
    throw new NxAgentToolError("unknown-outcome", "The request timed out after it was sent.");
  });
  const result = failed(await run("place_order", { sku: "X" }, request));
  assert.deepEqual(result.error, { code: "unknown-outcome", message: "The request timed out after it was sent." });
  assert.ok(result.usage!.operations! > 0);
});

test("an answer with no numeric status fails the call", async () => {
  for (const response of [undefined, null, "ok", { body: "ok" }, { status: "200", body: "ok" }, [200]]) {
    const { request } = recorder(() => response as never);
    const result = failed(await run("lookup_order", { orderId: "A1" }, request));
    assert.equal(result.error.code, "request-failed", JSON.stringify(response));
    assert.match(result.error.message, /no numeric 'status'/);
  }
});

test("a host forwards the arguments instead of the request, and the other process builds the same one", async () => {
  // The other process holds its own copy of the definition and trusts nothing but the tool's name
  // and the structured arguments it is sent.
  const stored = JSON.parse(JSON.stringify(definition)) as NormalizedAgent;
  const rebuilt: unknown[] = [];
  const api = (message: string): AgentHttpResponse => {
    const { toolName, args, callId } = JSON.parse(message) as { toolName: string; args: unknown; callId: string };
    const built = buildHttpRequest(stored.tools.find((tool) => tool.name === toolName)!, args);
    rebuilt.push(built.ok ? built.request : built.error);
    return { status: 200, body: { callId } };
  };
  const { calls, request } = recorder((call) => api(JSON.stringify({ toolName: call.toolName, args: call.arguments, callId: call.callId })));
  const result = succeeded(await run("place_order", { sku: "X", quantity: 3 }, request, { callId: "t9:0:0" }));
  assert.deepEqual(result.output, { status: 200, body: { callId: "t9:0:0" } });
  assert.deepEqual(rebuilt, [calls[0]!.request]);
});

// ---- evaluateHttpArguments ----------------------------------------------------------------------

test("a tool that is not as the package wrote it fails the evaluation, and is never a TypeError", () => {
  const lookupOrder = definitionOf("lookup_order");
  const damaged: readonly (readonly [unknown, RegExp])[] = [
    [null, /^The tool cannot be run: it is null, not a tool\.$/],
    [{ ...lookupOrder, config: undefined }, /its 'config' is undefined, not a record/],
    [{ ...lookupOrder, config: { ...lookupOrder.config, arguments: undefined } }, /its 'config\.arguments' is undefined/],
    [{ ...lookupOrder, config: { ...lookupOrder.config, contextParameters: undefined } }, /its 'config\.contextParameters' is undefined, not a list/],
  ];
  for (const [tool, message] of damaged) {
    const result = failed(evaluate(tool as NormalizedTool, { orderId: "A1" }));
    assert.equal(result.error.code, "invalid-request");
    assert.match(result.error.message, message);
    assert.equal("usage" in result, false);
  }
});

test("arguments are evaluated without sending: the structured arguments and the request", () => {
  const result = succeeded(evaluate("lookup_order", { orderId: "A1" }));
  assert.deepEqual(result.arguments, { pathParams: [{ name: "orderId", value: "A1" }], query: [] });
  assert.deepEqual(result.request, { method: "GET", url: "https://api.example.com/orders/A1", headers: {} });
  assert.ok(result.usage!.operations! > 0);
  assert.deepEqual(Object.keys(result), ["ok", "arguments", "request", "usage"]);
  // A host compares the names its function returned with the path's placeholders.
  assert.deepEqual(result.arguments.pathParams.map((parameter) => parameter.name), (definitionOf("lookup_order").config as { pathPlaceholders: string[] }).pathPlaceholders);
});

test("the evaluation gives the failure execute would, with the same usage", async () => {
  const { request } = recorder();
  const cases: readonly (readonly [string, unknown, Partial<ToolCallContext>])[] = [
    ["undeclared_parameter", { orderId: "A1" }, {}],
    ["body_on_get", { sku: "X" }, {}],
    ["lookup_order", { orderId: 7 }, {}],
    ["lookup_order", {}, {}],
    ["lookup_order", [], {}],
    ["order_at", { position: 0 }, {}],
    ["list_orders", { pages: 1000 }, {}],
    ["lookup_mine", { orderId: "A1" }, {}],
    ["lookup_order", { orderId: "A1" }, { callId: "" }],
    ["lookup_order", { orderId: "A1" }, { signal: AbortSignal.abort() }],
  ];
  for (const [name, input, context] of cases) {
    const executed = failed(await run(name, input, request, context));
    assert.deepEqual(failed(evaluate(name, input, context)), executed, name);
  }
});

test("the evaluation is given the context and the budget as a call is", () => {
  const result = succeeded(evaluate("lookup_mine", { orderId: "A1" }, { callId: "t7:3:0", context: chat }));
  assert.equal(result.request.url, "https://api.example.com/orders/A1?conversation=conv_9&call=t7%3A3%3A0");
  const budget = failed(evaluate("list_orders", { pages: 1000 }, {}, { runtime: { maxOperations: 5_000 } }));
  assert.deepEqual(budget.error.limit, { name: "maxOperations", value: 5_000 });
  assert.throws(() => evaluate("lookup_order", { orderId: "A1" }, {}, { maxArgumentsSize: -1 }), RangeError);
});

test("a tool that is not an http tool cannot be evaluated", () => {
  const lookupOrder = definitionOf("lookup_order");
  const functionTool = {
    ...lookupOrder,
    kind: "function",
    config: { function: (lookupOrder.config as { arguments: unknown }).arguments, contextParameters: [] },
  } as NormalizedTool;
  const result = failed(evaluate(functionTool, { orderId: "A1" }));
  assert.equal(result.error.code, "invalid-request");
  assert.match(result.error.message, /is of kind 'function', not 'http'/);
  // The builder, given the same tool, says the same thing in its own words and under the tool's name.
  const built = buildHttpRequest(functionTool, {});
  assert.equal(built.ok, false);
  assert.equal((built as { error: { message: string } }).error.message, `Tool 'lookup_order': its kind is the string "function", not 'http'.`);
});
