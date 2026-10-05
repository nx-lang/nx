import assert from "node:assert/strict";
import { after, test } from "node:test";

import { generateText, isStepCount, streamText, type ToolExecutionOptions, type ToolSet } from "ai";
import { MockLanguageModelV4, convertArrayToReadableStream } from "ai/test";

import { toAiSdkTools, type AiSdkToolCall, type AiSdkToolResult, type ToAiSdkToolsOptions } from "../src/ai-sdk.js";
import { createAgentTools, type AgentTool } from "../src/execute.js";
import { NxAgentToolError, type NormalizedAgent } from "../src/index.js";
import { normalizeAgent } from "../src/normalize.js";
import { chatToolContext, compileProgram, disposeCompiled } from "./nx-support.js";

after(disposeCompiled);

const compiled = compileProgram(`
/// A plan a team can buy.
type Plan = { name:string seats:int monthlyPrice:int }

/// Finds the plans that fit a team.
let findPlans(
  /// Number of people who need a seat.
  teamSize: int
): Plan* = { <Plan name="Team" seats={teamSize} monthlyPrice=20 /> }

/// Says which call of which conversation this is.
let whoAmI(context: ChatToolContext): string = { context.conversationId + "#" + context.callId }

/// Multiplies every pair of numbers below n.
let spin(n: int) = { for i in 0..n { for j in 0..n { i * j } } }

let root(): Agent = {
  <Agent name="support" tools={
    <FunctionTool function={findPlans} />
    <FunctionTool function={whoAmI} />
    <FunctionTool function={spin} />
    <WebSearchTool allowedDomains={ "docs.example.com" } />
  }>Be brief.</Agent>
}
`);

const normalized = normalizeAgent(compiled.agent(), { schemas: compiled.artifact, toolContextType: chatToolContext });
assert.equal(normalized.ok, true, JSON.stringify(normalized.diagnostics, null, 2));
const definition = (normalized as { definition: NormalizedAgent }).definition;
const tools: readonly AgentTool[] = createAgentTools(definition, compiled.program);
const executable = tools.filter((tool) => tool.execute !== undefined);

const plans = [{ $type: "Plan", name: "Team", seats: 5, monthlyPrice: 20 }];

function sdkOptions(toolCallId = "call_abc", extra: Partial<ToolExecutionOptions<unknown>> = {}): ToolExecutionOptions<unknown> {
  return { toolCallId, messages: [], context: undefined, ...extra };
}

/** Calls the adapted `execute` of one tool directly, as the SDK does. */
function call(toolSet: ToolSet, name: string, input: unknown, options = sdkOptions()): Promise<unknown> {
  const execute = toolSet[name]!.execute as (input: unknown, options: ToolExecutionOptions<unknown>) => Promise<unknown>;
  return execute(input, options);
}

function adapt(options: ToAiSdkToolsOptions = {}): ToolSet {
  return toAiSdkTools(executable, options);
}

async function rejection(promise: Promise<unknown>): Promise<unknown> {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("the promise resolved");
}

// ---- executable tools ---------------------------------------------------------------------------

test("an executable tool becomes a tool defined at run time, with its description and input schema and nothing else", () => {
  const toolSet = adapt();
  assert.deepEqual(Object.keys(toolSet), ["find_plans", "who_am_i", "spin"]);
  const tool = toolSet["find_plans"] as unknown as Record<string, unknown>;
  assert.equal(tool["type"], "dynamic");
  assert.equal(tool["description"], "Finds the plans that fit a team.");
  assert.deepEqual((tool["inputSchema"] as { jsonSchema: unknown }).jsonSchema, definition.tools[0]!.inputSchema);
  assert.equal(typeof tool["execute"], "function");
  // No strict mode, no approval, and no other option of a tool.
  assert.deepEqual(Object.keys(tool).sort(), ["description", "execute", "inputSchema", "type"]);
});

test("the SDK is given the input schema as it is stored, keywords the package does not know included", () => {
  const inputSchema = {
    type: "object",
    properties: { teamSize: { type: "integer", minimum: 1, maximum: 10, "x-nx-constraint": "1..10" } },
    required: ["teamSize"],
    additionalProperties: false,
  };
  const toolSet = toAiSdkTools([{ ...executable[0]!, inputSchema }]);
  const given = (toolSet["find_plans"] as unknown as { inputSchema: { jsonSchema: unknown } }).inputSchema.jsonSchema;
  assert.deepEqual(given, inputSchema);
  assert.equal(JSON.stringify(given), JSON.stringify(inputSchema));
});

test("a success returns the output to the SDK as it is, without the result's usage", async () => {
  const output = await call(adapt(), "find_plans", { teamSize: 5 });
  assert.deepEqual(output, plans);
  assert.ok(Array.isArray(output));
});

test("the callId is the SDK's toolCallId unless the host derives one", async () => {
  const context = () => ({ $type: "ChatToolContext", conversationId: "conv_9" });
  assert.equal(await call(adapt({ context }), "who_am_i", {}, sdkOptions("call_abc")), "conv_9#call_abc");

  const seen: AiSdkToolCall[] = [];
  const derived = adapt({
    context,
    callId: (info) => {
      seen.push(info);
      return "turn_7:1:0";
    },
  });
  const options = sdkOptions("call_xyz");
  assert.equal(await call(derived, "who_am_i", { anything: 1 }, options), "conv_9#turn_7:1:0");
  assert.equal(seen.length, 1);
  assert.deepEqual({ ...seen[0], options: undefined }, { toolName: "who_am_i", toolCallId: "call_xyz", input: { anything: 1 }, options: undefined });
  assert.equal(seen[0]!.options, options);
});

test("the host supplies the tool context record for each call", async () => {
  const toolSet = adapt({
    context: (info) => ({ $type: "ChatToolContext", conversationId: (info.options.context as { conversation: string }).conversation }),
  });
  assert.equal(await call(toolSet, "who_am_i", {}, sdkOptions("c1", { context: { conversation: "conv_1" } })), "conv_1#c1");
  assert.equal(await call(toolSet, "who_am_i", {}, sdkOptions("c2", { context: { conversation: "conv_2" } })), "conv_2#c2");

  // With no record supplied, a tool that needs one fails as it does when called directly.
  const error = await rejection(call(adapt(), "who_am_i", {}));
  assert.ok(error instanceof NxAgentToolError);
  assert.equal(error.code, "invalid-context");
});

test("a failure is thrown as the package's tool error, with the result's code", async () => {
  const budget = await rejection(call(adapt(), "spin", { n: 1000 }));
  assert.ok(budget instanceof NxAgentToolError);
  assert.equal(budget.name, "NxAgentToolError");
  assert.equal(budget.code, "resource-limit");

  // The model's own mistake is told to it: the SDK sends it the error's message. The diagnostics
  // and the limit are on the error for the host's own handlers.
  const input = await rejection(call(adapt(), "find_plans", { teamSize: "five" }));
  assert.ok(input instanceof NxAgentToolError);
  assert.equal(input.code, "invalid-input");
  assert.match(input.message, /teamSize/);
  assert.equal(input.diagnostics![0]!.code, "nx-ir-boundary-type");
  // A member the result does not have is not on the error either, not even as `undefined`.
  assert.equal("limit" in input, false);
  assert.deepEqual(Object.keys(input).sort(), ["code", "diagnostics", "name"]);
  const large = await rejection(call(adapt(), "find_plans", { teamSize: 5, note: "x".repeat(70_000) }));
  assert.ok(large instanceof NxAgentToolError);
  assert.equal(large.code, "invalid-input");
  assert.deepEqual(large.limit, { name: "maxArgumentsSize", value: 1_000 });
});

/** A tool whose every call ends in `error`, adapted, and the results `onResult` was handed. */
function failingWith(error: { code: string; message: string; diagnostics?: unknown; limit?: unknown }): { toolSet: ToolSet; events: AiSdkToolResult[] } {
  const events: AiSdkToolResult[] = [];
  const failing = { ...executable[0]!, name: "lookup_order", execute: async () => ({ ok: false, error }) } as unknown as AgentTool;
  return { toolSet: toAiSdkTools([failing], { onResult: (event) => void events.push(event) }), events };
}

test("the model is told why a call failed only when it can correct it; the host is told every time", async () => {
  const secret = "connect ECONNREFUSED https://billing.internal.example.com:8443/v1 (token sk-live-123)";
  const sentences: Record<string, string> = {
    "invalid-context": "Tool 'lookup_order' is not available for this call.",
    "evaluation-failed": "Tool 'lookup_order' failed while it ran.",
    "resource-limit": "Tool 'lookup_order' stopped at a limit on what one call may do.",
    "invalid-request": "Tool 'lookup_order' could not make a request from these arguments.",
    "request-failed": "Tool 'lookup_order' did not get an answer to its request.",
    aborted: "The call of tool 'lookup_order' was cancelled.",
  };
  for (const [code, sentence] of Object.entries(sentences)) {
    const error = { code, message: secret, diagnostics: [{ severity: "error", code: "nx-ir-evaluation", message: secret }], limit: { name: "maxOperations", value: 7 } };
    const { toolSet, events } = failingWith(error);
    const thrown = await rejection(call(toolSet, "lookup_order", {}));
    assert.ok(thrown instanceof NxAgentToolError, code);
    assert.equal(thrown.code, code);
    assert.equal(thrown.message, sentence);
    // Nothing of the reason is on the error: not in its text, not as a member that can be
    // listed or one that cannot, and not as its cause.
    assert.deepEqual(Object.keys(thrown).sort(), ["code", "name"], code);
    assert.deepEqual(Object.getOwnPropertyNames(thrown).sort(), ["code", "message", "name", "stack"], code);
    assert.equal(thrown.cause, undefined, code);
    assert.equal(String(thrown), `NxAgentToolError: ${sentence}`, code);
    assert.equal(JSON.stringify({ ...thrown, message: thrown.message, stack: thrown.stack }).includes("billing.internal"), false, code);
    // The host reads the whole failure where it reads every result.
    assert.deepEqual(events.map((event) => (event.result as { error: unknown }).error), [error], code);
  }

  // A code a host function chose comes with the message the host wrote for it.
  const own = { code: "order-not-found", message: "There is no order A1.", limit: { name: "orders", value: 0 } };
  const chosen = await rejection(call(failingWith(own).toolSet, "lookup_order", {}));
  assert.ok(chosen instanceof NxAgentToolError);
  assert.equal(chosen.code, "order-not-found");
  assert.equal(chosen.message, "There is no order A1.");
  assert.deepEqual(chosen.limit, own.limit);
  // A code that only resembles one of the package's, or names a member every object has, is the host's.
  for (const code of ["request-failed-upstream", "constructor", "toString", "__proto__"]) {
    const kept = await rejection(call(failingWith({ code, message: "As the host wrote it." }).toolSet, "lookup_order", {}));
    assert.ok(kept instanceof NxAgentToolError, code);
    assert.equal(kept.message, "As the host wrote it.", code);
  }
});

// An `http` tool whose request function fails, through the SDK: the scenario as a host meets it.
const shop = compileProgram(`
/// Looks an order up.
let lookupOrder(orderId: string): HttpArguments = { <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> /> }

/// Cancels an order.
let cancelOrder(orderId: string): HttpArguments = { <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> /> }

let billing = <HttpConnection name="billing" baseUrl="https://billing.internal.example.com/v1" />

let root(): Agent = {
  <Agent name="support" tools={
    <HttpTool connection={billing} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />
    <HttpTool connection={billing} method={HttpMethod.post} path="/orders/{orderId}/cancel" arguments={cancelOrder} />
  }>Be brief.</Agent>
}
`);

test("through generateText, what a request function threw does not reach the model, for a read and for a write", async () => {
  const normalizedShop = normalizeAgent(shop.agent(), { schemas: shop.artifact });
  assert.equal(normalizedShop.ok, true, JSON.stringify(normalizedShop.diagnostics, null, 2));
  const thrownText = "fetch failed: https://billing.internal.example.com/v1/orders/A1 (token sk-live-123)";
  const logged: unknown[] = [];
  // A request function written the way the README's is: the caught error is kept for the host,
  // and what is thrown under a code of the host's own is a sentence written for the model.
  const shopTools = createAgentTools((normalizedShop as { definition: NormalizedAgent }).definition, shop.program, {
    request: async ({ annotations }) => {
      try {
        throw new TypeError(thrownText);
      } catch (error) {
        logged.push(error);
        if (annotations.readOnlyHint) {
          throw error;
        }
        throw new NxAgentToolError("unknown-outcome", "The request was sent, and whether it was carried out is not known.");
      }
    },
  });
  const expected: Record<string, readonly [string, string]> = {
    lookup_order: ["request-failed", "NxAgentToolError: Tool 'lookup_order' did not get an answer to its request."],
    cancel_order: ["unknown-outcome", "NxAgentToolError: The request was sent, and whether it was carried out is not known."],
  };
  for (const [name, [code, told]] of Object.entries(expected)) {
    const events: AiSdkToolResult[] = [];
    const model = modelCalling(name, { orderId: "A1" });
    await generateText({ model, tools: toAiSdkTools(shopTools, { onResult: (event) => void events.push(event) }), prompt: "Do it.", stopWhen: isStepCount(2) });

    // The whole of what the model was sent about the failure.
    const sent = toolResultSent(model);
    assert.deepEqual(sent.output, { type: "error-text", value: told }, name);
    for (const part of ["billing.internal", "sk-live", "fetch failed"]) {
      assert.equal(JSON.stringify(model.doGenerateCalls[1]!.prompt).includes(part), false, `${name}: ${part}`);
    }
    // The host was handed the failure with its code, and for the read the text as it was thrown.
    assert.equal(events.length, 1);
    const { error } = events[0]!.result as { error: { code: string; message: string } };
    assert.equal(error.code, code, name);
    assert.equal(error.message, name === "lookup_order" ? thrownText : "The request was sent, and whether it was carried out is not known.");
  }
  assert.equal(logged.length, 2);
});

test("a host that wants the model told something else throws it from the result function", async () => {
  const failing = { ...executable[0]!, name: "lookup_order", execute: async () => ({ ok: false, error: { code: "request-failed", message: "upstream 503" } }) } as unknown as AgentTool;
  const toolSet = toAiSdkTools([failing], {
    onResult: ({ result }) => {
      if (!result.ok && result.error.code === "request-failed") {
        throw new NxAgentToolError("request-failed", "The order service is down. Tell the customer to try again later.");
      }
    },
  });
  const thrown = await rejection(call(toolSet, "lookup_order", {}));
  assert.ok(thrown instanceof NxAgentToolError);
  assert.equal(thrown.message, "The order service is down. Tell the customer to try again later.");
});

test("the SDK's abort signal reaches the tool", async () => {
  const error = await rejection(call(adapt(), "find_plans", { teamSize: 5 }, sdkOptions("c1", { abortSignal: AbortSignal.abort() })));
  assert.ok(error instanceof NxAgentToolError);
  assert.equal(error.code, "aborted");
});

// ---- onResult -----------------------------------------------------------------------------------

test("a host using the adapter reads what a call used, on a success and on a budget failure", async () => {
  const events: AiSdkToolResult[] = [];
  const toolSet = adapt({ onResult: (event) => void events.push(event), callId: () => "turn_7:1:0" });

  assert.deepEqual(await call(toolSet, "find_plans", { teamSize: 5 }), plans);
  assert.equal(events.length, 1);
  assert.equal(events[0]!.name, "find_plans");
  assert.equal(events[0]!.callId, "turn_7:1:0");
  assert.deepEqual(Object.keys(events[0]!), ["name", "callId", "result"]);
  const success = events[0]!.result;
  assert.equal(success.ok, true);
  assert.deepEqual((success as { output: unknown }).output, plans);
  assert.ok(success.usage!.operations! > 0);
  assert.equal(success.usage!.inputSize, 5);

  // The failure is handed over, with its usage, before the tool error is thrown.
  const error = await rejection(call(toolSet, "spin", { n: 1000 }));
  assert.ok(error instanceof NxAgentToolError);
  assert.equal(events.length, 2);
  const failure = events[1]!.result;
  assert.equal(failure.ok, false);
  assert.equal((failure as { error: { code: string } }).error.code, "resource-limit");
  assert.ok(failure.usage!.operations! > 90_000 && failure.usage!.operations! <= 100_000);
});

test("the adapter waits for the result function before it returns the output", async () => {
  const order: string[] = [];
  let stored!: () => void;
  const journal = new Promise<void>((resolve) => {
    stored = resolve;
  });
  const toolSet = adapt({
    onResult: async () => {
      order.push("journal begins");
      await journal;
      order.push("journal written");
    },
  });
  let settled = false;
  const pending = call(toolSet, "find_plans", { teamSize: 5 }).then((output) => {
    settled = true;
    order.push("output returned");
    return output;
  });
  // Give the call every chance to finish before the journal is written: it must not.
  for (let turn = 0; turn < 20; turn += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(settled, false, "the output was returned before the result function finished");
  assert.deepEqual(order, ["journal begins"]);
  stored();
  assert.deepEqual(await pending, plans);
  assert.deepEqual(order, ["journal begins", "journal written", "output returned"]);
});

test("what the result function throws, or rejects with, is the tool's error", async () => {
  const failure = new Error("the journal is unavailable");
  const throwing = adapt({
    onResult: () => {
      throw failure;
    },
  });
  assert.equal(await rejection(call(throwing, "find_plans", { teamSize: 5 })), failure);

  const rejecting = adapt({ onResult: () => Promise.reject(failure) });
  assert.equal(await rejection(call(rejecting, "find_plans", { teamSize: 5 })), failure);

  // On a failed call too: the host's error, and not the tool's, is what is thrown.
  assert.equal(await rejection(call(rejecting, "spin", { n: 1000 })), failure);
});

test("the result function is called once for each call, and not for a provider tool", async () => {
  let count = 0;
  const toolSet = toAiSdkTools(tools, { onResult: () => void (count += 1), providerTool: () => ({ type: "provider" }) as never });
  await call(toolSet, "find_plans", { teamSize: 5 });
  await call(toolSet, "find_plans", { teamSize: 6 });
  await rejection(call(toolSet, "find_plans", {}));
  assert.equal(count, 3);
});

// ---- provider tools -----------------------------------------------------------------------------

const providerWebSearch = { type: "provider", id: "openai.web_search", args: { filters: { allowedDomains: ["docs.example.com"] } }, inputSchema: {} } as unknown as ToolSet[string];

test("a provider tool is mapped by the host, which is given its definition", () => {
  const seen: AgentTool[] = [];
  const toolSet = toAiSdkTools(tools, {
    providerTool: (tool) => {
      seen.push(tool);
      return providerWebSearch;
    },
  });
  assert.deepEqual(Object.keys(toolSet), ["find_plans", "who_am_i", "spin", "web_search"]);
  assert.equal(toolSet["web_search"], providerWebSearch);
  assert.deepEqual(seen.map((tool) => [tool.name, tool.kind, tool.config]), [["web_search", "provider", { provider: "web_search", allowedDomains: ["docs.example.com"] }]]);
});

test("a provider tool the host maps to null is left out", () => {
  const toolSet = toAiSdkTools(tools, { providerTool: () => null });
  assert.deepEqual(Object.keys(toolSet), ["find_plans", "who_am_i", "spin"]);
});

test("an unmapped provider tool is an error naming the tool, not a silent omission", () => {
  for (const options of [{}, { providerTool: () => undefined }]) {
    assert.throws(
      () => toAiSdkTools(tools, options),
      (error) => error instanceof TypeError && error.message.includes("'web_search'"),
    );
  }
});

// ---- through the SDK ----------------------------------------------------------------------------

const usage = { inputTokens: { total: 1, noCache: 1, cacheRead: 0, cacheWrite: 0 }, outputTokens: { total: 1, text: 1, reasoning: 0 } };

/** A model that asks for one tool call and then, given its result, answers. */
function modelCalling(toolName: string, input: unknown): MockLanguageModelV4 {
  return new MockLanguageModelV4({
    doGenerate: [
      {
        content: [{ type: "tool-call", toolCallId: "call_1", toolName, input: JSON.stringify(input) }],
        finishReason: { unified: "tool-calls", raw: undefined },
        usage,
        warnings: [],
      },
      { content: [{ type: "text", text: "Done." }], finishReason: { unified: "stop", raw: undefined }, usage, warnings: [] },
    ],
  });
}

/** The tool result the model was sent in its second call: the part of the prompt's tool message. */
function toolResultSent(model: MockLanguageModelV4, calls = model.doGenerateCalls): { toolCallId: string; toolName: string; output: { type: string; value: unknown } } {
  assert.equal(calls.length, 2, "the model was not called again with the tool's result");
  const message = calls[1]!.prompt.find((entry) => entry.role === "tool");
  assert.ok(message !== undefined, "the second call's prompt has no tool message");
  const part = (message.content as readonly { type: string }[]).find((entry) => entry.type === "tool-result");
  assert.ok(part !== undefined, "the tool message has no tool result");
  return part as never;
}

test("through generateText, the model receives the function's canonical result", async () => {
  const model = modelCalling("find_plans", { teamSize: 5 });
  const events: AiSdkToolResult[] = [];
  const result = await generateText({
    model,
    tools: toAiSdkTools(tools, { providerTool: () => null, onResult: (event) => void events.push(event) }),
    prompt: "Which plan fits a team of five?",
    stopWhen: isStepCount(2),
  });
  assert.equal(result.text, "Done.");

  // The SDK invoked the tool with the parsed input.
  assert.equal(events.length, 1);
  assert.equal(events[0]!.callId, "call_1");
  assert.equal(events[0]!.result.ok, true);

  // What the model was sent back is the function's result as it is: an array, not wrapped.
  const sent = toolResultSent(model);
  assert.equal(sent.toolCallId, "call_1");
  assert.equal(sent.toolName, "find_plans");
  assert.deepEqual(sent.output, { type: "json", value: plans });

  // The model was offered the tool with the definition's description and input schema.
  const offered = model.doGenerateCalls[0]!.tools!.find((tool) => tool.name === "find_plans") as unknown as { description: string; inputSchema: unknown };
  assert.equal(offered.description, "Finds the plans that fit a team.");
  assert.deepEqual(offered.inputSchema, definition.tools[0]!.inputSchema);
});

test("through generateText, a failing tool reaches the model as a tool error", async () => {
  const model = modelCalling("spin", { n: 1000 });
  const result = await generateText({
    model,
    tools: toAiSdkTools(tools, { providerTool: () => null }),
    prompt: "Spin.",
    stopWhen: isStepCount(2),
  });
  assert.equal(result.text, "Done.");

  const errors = result.steps[0]!.content.filter((part) => part.type === "tool-error");
  assert.equal(errors.length, 1);
  const thrown = (errors[0] as { error: unknown }).error;
  assert.ok(thrown instanceof NxAgentToolError);
  assert.equal(thrown.code, "resource-limit");

  const sent = toolResultSent(model);
  assert.equal(sent.toolName, "spin");
  assert.match(sent.output.type, /^error-/);
  // The model is told what kind of failure it was, and not the budget or how it was spent.
  assert.deepEqual(sent.output, { type: "error-text", value: "NxAgentToolError: Tool 'spin' stopped at a limit on what one call may do." });
  assert.equal(/100000|operations|budget/.test(JSON.stringify(sent.output)), false, JSON.stringify(sent.output));
});

test("a function tool is given to streamText, and the model receives the function's canonical result", async () => {
  const model = new MockLanguageModelV4({
    doStream: [
      {
        stream: convertArrayToReadableStream([
          { type: "stream-start", warnings: [] },
          { type: "tool-call", toolCallId: "call_1", toolName: "find_plans", input: JSON.stringify({ teamSize: 5 }) },
          { type: "finish", finishReason: { unified: "tool-calls", raw: undefined }, usage },
        ]),
      },
      {
        stream: convertArrayToReadableStream([
          { type: "stream-start", warnings: [] },
          { type: "text-start", id: "t1" },
          { type: "text-delta", id: "t1", delta: "Done." },
          { type: "text-end", id: "t1" },
          { type: "finish", finishReason: { unified: "stop", raw: undefined }, usage },
        ]),
      },
    ],
  });
  const inputs: unknown[] = [];
  const result = streamText({
    model,
    tools: toAiSdkTools(tools, { providerTool: () => null, callId: (call) => (inputs.push(call.input), call.toolCallId) }),
    prompt: "Which plan fits a team of five?",
    stopWhen: isStepCount(2),
  });
  assert.equal(await result.text, "Done.");
  // The SDK invoked the tool's execute with the parsed input, and sent the model its result as it is.
  assert.deepEqual(inputs, [{ teamSize: 5 }]);
  const sent = toolResultSent(model, model.doStreamCalls);
  assert.equal(sent.toolName, "find_plans");
  assert.deepEqual(sent.output, { type: "json", value: plans });
});
