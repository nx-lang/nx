import assert from "node:assert/strict";
import { after, test } from "node:test";

import { measureInputSize, type NxRuntimeUsage } from "@nx-lang/ir-runtime";

import {
  NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE,
  NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE,
  NX_AGENT_DEFAULT_MAX_OPERATIONS,
  checkAgentTools,
  createAgentTools,
  type AgentTool,
  type CreateAgentToolsOptions,
  type ToolCallContext,
  type ToolResult,
} from "../src/execute.js";
import { NxAgentError, NxAgentToolError, type NormalizedAgent, type NormalizedTool } from "../src/index.js";
import { normalizeAgent, type NormalizeAgentOptions } from "../src/normalize.js";
import { chatToolContext, compileProgram, disposeCompiled } from "./nx-support.js";

after(disposeCompiled);

const readOnly = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false };
const builtin: NormalizeAgentOptions["toolTypes"] = {
  RecordSearchTool: (value) => ({
    name: "search_records",
    description: "Searches records.",
    inputSchema: { type: "object", properties: { query: { type: "string" } }, required: ["query"], additionalProperties: false },
    annotations: readOnly,
    kind: "builtin",
    config: { builtin: "record_search", recordKind: (value as unknown as { recordKind: string }).recordKind },
  }),
};

// A name of more than 64 code units, which costs one as a field name of a call's arguments.
const longName = `context${"X".repeat(63)}`;

const compiled = compileProgram(`
/// A plan a team can buy.
type Plan = { name:string seats:int monthlyPrice:int }

/// Finds the plans that fit a team.
let findPlans(teamSize: int, maxMonthlyPrice?: int): Plan* = { <Plan name="Team" seats={teamSize} monthlyPrice=20 /> }

/// Divides ten by a number.
let divide(n: int): int = { 10 / n }

/// Multiplies every pair of numbers below n.
let spin(n: int) = { for i in 0..n { for j in 0..n { i * j } } }

/// Says which conversation the call belongs to.
let whoAmI(context: ChatToolContext): string = { context.conversationId }

/// Echoes the call's id; any host context will do.
let callIdOf(context: ToolContext): string = { context.callId }

/// Looks an order up for the conversation.
let orderFor(orderId: string, context: ChatToolContext): string = { orderId + "@" + context.conversationId + "#" + context.callId }

type Context = ChatToolContext

/// Says which conversation the call belongs to, through an alias of the context type.
let whoAmIAliased(context: Context): string = { context.conversationId }

/// Weighs a note for the conversation.
let weigh(context: ChatToolContext, note?: string): string = { context.conversationId }

/// Takes the context twice.
let twice(first: ChatToolContext, second: ToolContext, note?: string): string = { first.callId + second.callId }

/// Takes the context under a long name.
let longNamed(${longName}: ChatToolContext, note?: string): string = { ${longName}.callId }

/// Looks an order up by its identifier.
let lookupOrder(orderId: string): HttpArguments = { <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> /> }

let shop = <HttpConnection name="shop" baseUrl="https://api.example.com" />

let root(): Agent = {
  <Agent name="support" tools={
    <FunctionTool function={findPlans} />
    <FunctionTool function={divide} />
    <FunctionTool function={spin} />
    <FunctionTool function={whoAmI} />
    <FunctionTool function={callIdOf} />
    <FunctionTool function={orderFor} />
    <FunctionTool function={weigh} />
    <FunctionTool function={twice} />
    <FunctionTool function={longNamed} />
    <WebSearchTool />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />
    <RecordSearchTool recordKind="company" />
    <FunctionTool function={whoAmIAliased} />
  }>Be brief.</Agent>
}
`);

const normalized = normalizeAgent(compiled.agent(), { schemas: compiled.artifact, toolContextType: chatToolContext, toolTypes: builtin });
assert.equal(normalized.ok, true, JSON.stringify(normalized.diagnostics, null, 2));
// The definition as a host has it at run time: stored as text and parsed back.
const definition = JSON.parse(JSON.stringify((normalized as { definition: NormalizedAgent }).definition)) as NormalizedAgent;

const hostOptions: CreateAgentToolsOptions = {
  request: async () => ({ status: 200, body: null }),
  executors: { builtin: async () => "found" },
};

function toolsWith(options: CreateAgentToolsOptions = {}): Map<string, AgentTool> {
  return new Map(createAgentTools(definition, compiled.program, { ...hostOptions, ...options }).map((tool) => [tool.name, tool]));
}

const tools = toolsWith();

function run(name: string, input: unknown, context: Partial<ToolCallContext> = {}, from = tools): Promise<ToolResult> {
  return from.get(name)!.execute!(input, { callId: "t1:0:0", ...context } as ToolCallContext);
}

function failed(result: ToolResult): Extract<ToolResult, { ok: false }> {
  assert.equal(result.ok, false, JSON.stringify(result));
  return result as Extract<ToolResult, { ok: false }>;
}

function succeeded(result: ToolResult): Extract<ToolResult, { ok: true }> {
  assert.equal(result.ok, true, JSON.stringify(result));
  return result as Extract<ToolResult, { ok: true }>;
}

function creationError(create: () => unknown): NxAgentError {
  let thrown: unknown;
  try {
    create();
  } catch (error) {
    thrown = error;
  }
  assert.ok(thrown instanceof NxAgentError, `expected an NxAgentError, got ${String(thrown)}`);
  return thrown;
}

/** A model's input of exactly `size`, with the premise checked: the record, the string and its length. */
function inputOfSize(size: number): { note: string } {
  const input = { note: "x".repeat(64 * (size - 2)) };
  assert.equal(measureInputSize(input), size);
  return input;
}

/** A host's context record that measures exactly `size` once `callId` is set on it. */
function contextOfSize(size: number): { $type: string; conversationId: string } {
  const record = { $type: "ChatToolContext", conversationId: "c".repeat(64 * (size - 3)) };
  assert.equal(measureInputSize({ ...record, callId: "t1:0:0" }), size);
  return record;
}

const chat = { $type: "ChatToolContext", conversationId: "conv_9" };

// ---- creation -----------------------------------------------------------------------------------

test("one tool is returned for each definition, in order, with its fields unchanged", () => {
  const created = createAgentTools(definition, compiled.program, hostOptions);
  assert.deepEqual(
    created.map((tool) => tool.name),
    definition.tools.map((tool) => tool.name),
  );
  for (const [index, tool] of created.entries()) {
    const { execute, ...fields } = tool;
    assert.deepEqual(fields, definition.tools[index]);
    assert.equal(typeof execute, tool.kind === "provider" ? "undefined" : "function", tool.name);
  }
});

test("a provider tool has no execute", () => {
  const webSearch = tools.get("web_search")!;
  assert.equal(webSearch.kind, "provider");
  assert.equal("execute" in webSearch, false);
});

test("the tool list alone is accepted in place of the definition", () => {
  const created = createAgentTools(definition.tools, compiled.program, hostOptions);
  assert.equal(created.length, definition.tools.length);
});

test("a definition that is not as the package wrote it is reported with its path, and never as a TypeError", () => {
  const findPlans = definition.tools.find((tool) => tool.name === "find_plans")!;
  const lookupOrder = definition.tools.find((tool) => tool.kind === "http")!;
  const reference = (findPlans.config as { function: unknown }).function;
  const damaged: readonly (readonly [unknown, string, RegExp])[] = [
    [null, "", /^The definition is null, not a definition or a list of tools\.$/],
    ["{}", "", /is the string "\{\}"/],
    [{ ...definition, tools: {} }, "tools", /'tools' is a record with no '\$type', not a list/],
    [{ ...definition, tools: undefined }, "tools", /'tools' is undefined, not a list/],
    [{ ...definition, tools: [null] }, "tools[0]", /it is null, not a tool/],
    [[findPlans, "find_plans"], "tools[1]", /it is the string "find_plans", not a tool/],
    [[{ ...findPlans, name: 7 }], "tools[0]", /its 'name' is the number 7, not a string/],
    [[{ ...findPlans, kind: undefined }], "tools[0]", /its 'kind' is undefined, not a string/],
    [[{ ...findPlans, config: undefined }], "tools[0]", /its 'config' is undefined, not a record/],
    [[{ ...findPlans, config: { contextParameters: [] } }], "tools[0]", /its 'config\.function' is undefined, not the module and the name of a function/],
    [[{ ...findPlans, config: { function: { module: "main.nx" }, contextParameters: [] } }], "tools[0]", /its 'config\.function' is a record/],
    [[{ ...findPlans, config: { function: reference } }], "tools[0]", /its 'config\.contextParameters' is undefined, not a list/],
    [[{ ...findPlans, config: { function: reference, contextParameters: [{ type: {} }] } }], "tools[0]", /holds something other than a parameter with a 'name'/],
    [[{ ...lookupOrder, config: { ...lookupOrder.config, arguments: "lookupOrder" } }], "tools[0]", /its 'config\.arguments' is the string "lookupOrder"/],
    [[{ ...lookupOrder, config: { ...lookupOrder.config, connection: undefined } }], "tools[0]", /its 'config\.connection' is undefined, not a connection with a 'name'/],
  ];
  for (const [value, path, message] of damaged) {
    const checked = checkAgentTools(value as never, compiled.program);
    assert.deepEqual(
      checked.map((diagnostic) => [diagnostic.severity, diagnostic.code, diagnostic.path]),
      [["error", "nx-agent-invalid-definition", path]],
      JSON.stringify(value),
    );
    assert.match(checked[0]!.message, message);
    assert.deepEqual(creationError(() => createAgentTools(value as never, compiled.program, hostOptions)).diagnostics, checked);
  }

  // A tool that still has its name is reported under it, beside everything that is wrong with
  // the others: the error holds every problem, as it does for a definition that is not damaged.
  const mixed = [
    { ...findPlans, config: null },
    { ...findPlans, name: "gone", config: { function: { module: "main.nx", name: "gone" }, contextParameters: [] } },
    lookupOrder,
  ];
  const error = creationError(() => createAgentTools(mixed as never, compiled.program));
  assert.deepEqual(
    error.diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.path, diagnostic.tool]),
    [
      ["nx-agent-invalid-definition", "tools[0]", "find_plans"],
      ["nx-agent-unknown-function", "tools[1]", "gone"],
      ["nx-agent-missing-request-function", "tools[2]", "lookup_order"],
    ],
  );
  assert.equal("tool" in checkAgentTools([null] as never, compiled.program)[0]!, false);

  // A provider tool and a host's own tool have nothing the package reads before a call.
  assert.deepEqual(checkAgentTools([{ name: "web_search", kind: "provider" }, { name: "search_records", kind: "builtin" }] as never, compiled.program), []);
});

test("an unknown format version is refused, naming the version given and the versions supported", () => {
  for (const formatVersion of [2, 0, "1", undefined]) {
    const error = creationError(() => createAgentTools({ ...definition, formatVersion } as never, compiled.program, hostOptions));
    assert.deepEqual(
      error.diagnostics.map((diagnostic) => [diagnostic.severity, diagnostic.code, diagnostic.path]),
      [["error", "nx-agent-unsupported-format", "formatVersion"]],
    );
    assert.ok(error.diagnostics[0]!.message.includes(`format version ${JSON.stringify(formatVersion) ?? "undefined"}`), error.message);
    assert.match(error.diagnostics[0]!.message, /this package supports 1\./);
    assert.equal(error.message, error.diagnostics[0]!.message);
  }
});

test("a function missing from the program is refused at creation, naming the tool, the module and the function", () => {
  const withFunction = (reference: { module: string; name: string }): NormalizedAgent => ({
    ...definition,
    tools: definition.tools.map((tool) => (tool.name === "find_plans" ? ({ ...tool, config: { function: reference, contextParameters: [] } } as NormalizedTool) : tool)),
  });
  const cases = [
    { module: "main.nx", name: "findPlanz" },
    { module: "other.nx", name: "findPlans" },
    // A declaration of that name that is not a function.
    { module: "main.nx", name: "Plan" },
    { module: "main.nx", name: "shop" },
  ];
  for (const reference of cases) {
    let created: unknown;
    const error = creationError(() => {
      created = createAgentTools(withFunction(reference), compiled.program, hostOptions);
    });
    assert.equal(created, undefined, "no tools are returned");
    assert.deepEqual(error.diagnostics, [
      {
        severity: "error",
        code: "nx-agent-unknown-function",
        message: `Tool 'find_plans' names the function '${reference.name}' of module '${reference.module}', which the linked program does not declare.`,
        path: "tools[0]",
        tool: "find_plans",
        declaration: { module: reference.module },
      },
    ]);
  }
});

test("an http tool's arguments function is looked up as a function tool's function is", () => {
  const stale: NormalizedAgent = {
    ...definition,
    tools: definition.tools.map((tool) =>
      tool.name === "lookup_order" ? ({ ...tool, config: { ...tool.config, arguments: { module: "main.nx", name: "gone" } } } as NormalizedTool) : tool,
    ),
  };
  const error = creationError(() => createAgentTools(stale, compiled.program, hostOptions));
  assert.deepEqual(error.diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.tool]), [["nx-agent-unknown-function", "lookup_order"]]);
});

test("a kind with no executor is refused at creation, naming the tool and the kind", () => {
  const error = creationError(() => createAgentTools(definition, compiled.program, { request: hostOptions.request! }));
  assert.deepEqual(error.diagnostics, [
    {
      severity: "error",
      code: "nx-agent-missing-executor",
      message: "Tool 'search_records' is of kind 'builtin', and no executor is registered for that kind.",
      path: "tools[11]",
      tool: "search_records",
    },
  ]);
  // An executor under an inherited property name is not an executor.
  const inherited: NormalizedAgent = { ...definition, tools: [{ ...definition.tools[11]!, kind: "toString" } as NormalizedTool] };
  assert.equal(creationError(() => createAgentTools(inherited, compiled.program, {})).diagnostics[0]!.code, "nx-agent-missing-executor");
});

test("an http tool with no request function is refused at creation", () => {
  const error = creationError(() => createAgentTools(definition, compiled.program, { executors: hostOptions.executors! }));
  assert.deepEqual(error.diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.path, diagnostic.tool]), [
    ["nx-agent-missing-request-function", "tools[10]", "lookup_order"],
  ]);
});

test("every problem is thrown together, before any tool runs", () => {
  const broken: NormalizedAgent = {
    ...definition,
    tools: definition.tools.map((tool) =>
      tool.name === "divide" ? ({ ...tool, config: { function: { module: "main.nx", name: "gone" }, contextParameters: [] } } as NormalizedTool) : tool,
    ),
  };
  const error = creationError(() => createAgentTools(broken, compiled.program));
  assert.deepEqual(error.diagnostics.map((diagnostic) => diagnostic.code), [
    "nx-agent-unknown-function",
    "nx-agent-missing-request-function",
    "nx-agent-missing-executor",
  ]);
});

test("a host checks a definition against the program it linked, with no executors and no request function", () => {
  assert.deepEqual(checkAgentTools(definition, compiled.program), []);
  assert.deepEqual(checkAgentTools(definition.tools, compiled.program), []);
  const stale: NormalizedAgent = {
    ...definition,
    tools: definition.tools.map((tool) =>
      tool.name === "find_plans" ? ({ ...tool, config: { function: { module: "main.nx", name: "gone" }, contextParameters: [] } } as NormalizedTool) : tool,
    ),
  };
  assert.deepEqual(checkAgentTools(stale, compiled.program).map((diagnostic) => [diagnostic.code, diagnostic.path, diagnostic.tool]), [
    ["nx-agent-unknown-function", "tools[0]", "find_plans"],
  ]);
  assert.deepEqual(checkAgentTools({ ...definition, formatVersion: 7 } as never, compiled.program).map((diagnostic) => diagnostic.code), [
    "nx-agent-unsupported-format",
  ]);
});

test("the package's own kinds cannot be given an executor", () => {
  for (const kind of ["function", "http", "provider"]) {
    assert.throws(
      () => createAgentTools(definition, compiled.program, { ...hostOptions, executors: { ...hostOptions.executors, [kind]: async () => "mine" } }),
      (error) => error instanceof TypeError && error.message.includes(`'${kind}'`),
    );
  }
});

test("a limit that is not a non-negative safe integer is refused, so none can be turned off", () => {
  for (const value of [-1, 1.5, Number.POSITIVE_INFINITY, Number.NaN]) {
    assert.throws(() => createAgentTools(definition, compiled.program, { ...hostOptions, maxArgumentsSize: value }), RangeError);
    assert.throws(() => createAgentTools(definition, compiled.program, { ...hostOptions, maxContextSize: value }), RangeError);
    assert.throws(() => createAgentTools(definition, compiled.program, { ...hostOptions, runtime: { maxOperations: value } }), RangeError);
  }
});

// ---- the call context ---------------------------------------------------------------------------

test("an aborted call does nothing", async () => {
  let called = false;
  const from = toolsWith({
    executors: {
      builtin: async () => {
        called = true;
        return "found";
      },
    },
  });
  const signal = AbortSignal.abort();
  for (const name of ["find_plans", "lookup_order", "search_records"]) {
    const result = failed(await run(name, { teamSize: 5, orderId: "A1", query: "x" }, { signal }, from));
    assert.equal(result.error.code, "aborted");
    assert.equal("usage" in result, false);
  }
  assert.equal(called, false);
});

test("a call with no callId, or an empty one, is an invalid context", async () => {
  for (const callId of [undefined, "", 7, null]) {
    const result = failed(await tools.get("find_plans")!.execute!({ teamSize: 5 }, { callId } as never));
    assert.equal(result.error.code, "invalid-context");
    assert.match(result.error.message, /needs a 'callId'/);
    assert.equal("usage" in result, false);
  }
  const noContext = failed(await tools.get("find_plans")!.execute!({ teamSize: 5 }, undefined as never));
  assert.equal(noContext.error.code, "invalid-context");
});

test("the host supplies identity: the record is passed with callId set", async () => {
  const result = succeeded(await run("order_for", { orderId: "A1" }, { callId: "t1:0:0", context: chat }));
  assert.equal(result.output, "A1@conv_9#t1:0:0");
});

test("the call's callId replaces one the record held, and the host's record is not changed", async () => {
  const record = { $type: "ChatToolContext", conversationId: "conv_9", callId: "stale" };
  assert.equal(succeeded(await run("order_for", { orderId: "A1" }, { callId: "t2:1:0", context: record })).output, "A1@conv_9#t2:1:0");
  assert.equal(record.callId, "stale");
});

test("a base-typed parameter receives the host's subtype", async () => {
  assert.equal(succeeded(await run("call_id_of", {}, { callId: "t1:0:0", context: chat })).output, "t1:0:0");
  assert.equal(succeeded(await run("who_am_i", {}, { context: chat })).output, "conv_9");
  // An optional field a host set to `undefined`, which its generated type allows, is one that is
  // absent: the runtime reads the member as one left out.
  const withUndefined = { ...chat, contactEmail: undefined } as unknown as ToolCallContext["context"];
  const absent = succeeded(await run("who_am_i", {}, { context: withUndefined }));
  assert.equal(absent.output, "conv_9");
  assert.equal(absent.usage!.inputSize, succeeded(await run("who_am_i", {}, { context: chat })).usage!.inputSize);
  // The host's own record is not changed by the call.
  assert.deepEqual(Object.keys(withUndefined!), [...Object.keys(chat), "contactEmail"]);
  // A record a host built with a class is read as the record its own members make. A field the
  // class declares and does not set is an own member holding `undefined`, and is left out too.
  class ChatContext {
    public readonly $type = "ChatToolContext";
    public contactEmail: string | undefined;
    public constructor(public readonly conversationId: string) {}
  }
  const built = new ChatContext("conv_9");
  // The premise: the unset field is an own member of the instance, holding `undefined`.
  assert.deepEqual(Object.keys(built).sort(), ["$type", "contactEmail", "conversationId"]);
  assert.equal(built.contactEmail, undefined);
  const fromClass = succeeded(await run("who_am_i", {}, { context: built as never }));
  assert.equal(fromClass.output, "conv_9");
  assert.equal(fromClass.usage!.inputSize, absent.usage!.inputSize);
  // A record that holds itself, and one nested very deeply, are refused as too large. Neither is
  // read without end, and neither rejects.
  const cyclic: Record<string, unknown> = { ...chat };
  cyclic["self"] = cyclic;
  let deep: Record<string, unknown> = { ...chat };
  for (let depth = 0; depth < 20_000; depth += 1) {
    deep = { ...chat, inner: deep };
  }
  for (const tooLarge of [cyclic, deep, { ...chat, items: new Array<number>(2_000_000).fill(1) }]) {
    const refused = failed(await run("who_am_i", {}, { context: tooLarge as never }));
    assert.equal(refused.error.code, "resource-limit");
    assert.deepEqual(refused.error.limit, { name: "maxContextSize", value: 1_000 });
    assert.equal("usage" in refused, false);
  }
  // A member that is `null` is a value, and is passed as it is.
  assert.equal(succeeded(await run("who_am_i", {}, { context: { ...chat, contactEmail: null } })).output, "conv_9");
  // And a parameter declared through an alias of the host's type receives it too.
  assert.equal(succeeded(await run("who_am_i_aliased", { context: "forged" }, { context: chat })).output, "conv_9");
});

test("below the top level the runtime reads the record: an instance of a class or a Date there is the host's mistake", async () => {
  const nested = compileProgram(`
type Inner = { a:string b?:string }
type NestedContext extends ToolContext = { inner?:Inner extra?:object }

/// Reads the nested part of the context.
let readInner(context: NestedContext): string = { "read" }

let root(): Agent = { <Agent name="support" tools={ <FunctionTool function={readInner} /> }>Be brief.</Agent> }
`);
  const normalizedNested = normalizeAgent(nested.agent(), { schemas: nested.artifact, toolContextType: { module: "main.nx", name: "NestedContext" } });
  assert.equal(normalizedNested.ok, true, JSON.stringify(normalizedNested.diagnostics, null, 2));
  const [readInner] = createAgentTools((normalizedNested as { definition: NormalizedAgent }).definition, nested.program);
  const call = (members: Record<string, unknown>): Promise<ToolResult> =>
    readInner!.execute!({}, { callId: "t1:0:0", context: { $type: "NestedContext", ...members } as never });
  class Inner {
    public readonly $type = "Inner";
    public b: string | undefined;
    public constructor(public readonly a: string) {}
  }

  // A plain object below the top level has its `undefined` member left out, by the runtime.
  assert.equal(succeeded(await call({ inner: { $type: "Inner", a: "x", b: undefined } })).output, "read");
  assert.equal(succeeded(await call({ inner: undefined, extra: { deep: [{ gone: undefined }] } })).output, "read");
  // An instance of a class there is not a record, whatever its members hold. It is in the record
  // the host filled in, so the mistake is the host's, and the diagnostic says where it is.
  for (const instance of [new Inner("x"), Object.assign(new Inner("x"), { b: "y" })]) {
    const refused = failed(await call({ inner: instance }));
    assert.equal(refused.error.code, "invalid-context");
    assert.equal(refused.error.diagnostics![0]!.code, "nx-ir-boundary-type");
    assert.equal(refused.error.diagnostics![0]!.argument, "context");
    assert.match(refused.error.message, /Expected context\.inner to be .*got an instance of Inner, which is not a plain object/);
  }
  // The same members as a plain object are a record.
  assert.equal(succeeded(await call({ inner: { ...new Inner("x") } })).output, "read");
  // A value that is no value at all fails the same way, in a field typed `object` too.
  const dated = failed(await call({ extra: { seen: [new Date(0)] } }));
  assert.equal(dated.error.code, "invalid-context");
  assert.equal(dated.error.diagnostics![0]!.argument, "context");
  assert.match(dated.error.message, /Expected context\.extra\.seen\[0\] to be .*got an instance of Date/);
});

test("the model cannot supply the context", async () => {
  const result = succeeded(await run("order_for", { orderId: "A1", context: { $type: "ChatToolContext", conversationId: "other", callId: "forged" } }, { context: chat }));
  assert.equal(result.output, "A1@conv_9#t1:0:0");
  // Whatever the model puts under the name, of any shape.
  for (const forged of ["other", null, 7, [{ conversationId: "other" }]]) {
    assert.equal(succeeded(await run("order_for", { orderId: "A1", context: forged }, { context: chat })).output, "A1@conv_9#t1:0:0");
  }
});

test("a missing context record is reported, naming the parameter, without calling the function", async () => {
  const missing = failed(await run("order_for", { orderId: "A1" }));
  assert.equal(missing.error.code, "invalid-context");
  assert.match(missing.error.message, /for its parameter 'context'; the call supplied none/);
  assert.equal("usage" in missing, false);

  for (const context of [{ conversationId: "conv_9" }, { $type: 7, conversationId: "conv_9" }, "conv_9", [chat], null]) {
    const result = failed(await run("order_for", { orderId: "A1" }, { context: context as never }));
    assert.equal(result.error.code, "invalid-context", JSON.stringify(context));
    assert.equal("usage" in result, false);
  }
  assert.match(failed(await run("twice", {})).error.message, /parameter 'first', 'second'/);
});

test("a tool with no context parameter ignores the record", async () => {
  const result = succeeded(await run("find_plans", { teamSize: 5 }, { context: { anything: true } as never }));
  assert.deepEqual(result.output, [{ $type: "Plan", name: "Team", seats: 5, monthlyPrice: 20 }]);
});

test("a context record that does not fit its declared type is the host's mistake", async () => {
  // A required field is missing. The runtime names the argument, and the context filled it in.
  const result = failed(await run("order_for", { orderId: "A1" }, { context: { $type: "ChatToolContext" } }));
  assert.equal(result.error.code, "invalid-context");
  assert.deepEqual(result.error.diagnostics, [
    {
      severity: "error",
      code: "nx-ir-boundary-field",
      message: "Missing required context field 'conversationId'.",
      declaration: "host/Chat.nx::ChatToolContext",
      argument: "context",
    },
  ]);
  assert.match(result.error.message, /conversationId/);
  // The call reached the runtime, so it reports what it used.
  assert.equal(typeof result.usage!.operations, "number");
  assert.equal(typeof result.usage!.inputSize, "number");

  // A field of the wrong type, and one the type does not declare.
  for (const context of [
    { $type: "ChatToolContext", conversationId: 9 },
    { $type: "ChatToolContext", conversationId: "conv_9", tenant: "acme" },
  ]) {
    const unfit = failed(await run("order_for", { orderId: "A1" }, { context: context as never }));
    assert.equal(unfit.error.code, "invalid-context", JSON.stringify(context));
    assert.equal(unfit.error.diagnostics![0]!.argument, "context");
    assert.equal(typeof unfit.usage!.operations, "number");
  }
});

test("a context record of another type is the host's mistake", async () => {
  // A type that is not the declared one and does not extend it: another of the host's own, and one
  // the program does not have.
  for (const context of [
    { $type: "AuditToolContext", actor: "kai" },
    { $type: "NoSuchContext", conversationId: "conv_9" },
  ]) {
    const result = failed(await run("order_for", { orderId: "A1" }, { context }));
    assert.equal(result.error.code, "invalid-context", JSON.stringify(context));
    assert.equal(result.error.diagnostics![0]!.code, "nx-ir-boundary-type");
    assert.equal(result.error.diagnostics![0]!.argument, "context");
    assert.equal(typeof result.usage!.operations, "number");
  }
  // Whichever parameter of a function that takes the context twice refuses it.
  const twice = failed(await run("twice", {}, { context: { $type: "ChatToolContext" } }));
  assert.equal(twice.error.code, "invalid-context");
  assert.equal(twice.error.diagnostics![0]!.argument, "first");
});

test("a wrong-typed argument beside a context that fits is the model's mistake", async () => {
  const result = failed(await run("order_for", { orderId: 7 }, { context: chat }));
  assert.equal(result.error.code, "invalid-input");
  assert.equal(result.error.diagnostics![0]!.code, "nx-ir-boundary-type");
  assert.equal(result.error.diagnostics![0]!.argument, "orderId");
  // A missing one too, with nothing wrong in the context.
  const missing = failed(await run("order_for", {}, { context: chat }));
  assert.equal(missing.error.code, "invalid-input");
  assert.equal(missing.error.diagnostics![0]!.argument, "orderId");
});

test("a field default that does not fit its field is an evaluation failure, though its code is a boundary code", async () => {
  // The checker accepts this default because it reads another field (`specs/future.md`). The
  // runtime fills it in while it checks `req`, and the failure is not in what the model sent.
  const defaulted = compileProgram(`
type Req = { n:int label:string = { n } }

/// Reads the label of a request.
let labelOf(req: Req): string = { req.label }

let root(): Agent = { <Agent name="support" tools={ <FunctionTool function={labelOf} /> }>Be brief.</Agent> }
`);
  const normalizedDefaulted = normalizeAgent(defaulted.agent(), { schemas: defaulted.artifact });
  assert.equal(normalizedDefaulted.ok, true, JSON.stringify(normalizedDefaulted.diagnostics, null, 2));
  const [labelOf] = createAgentTools((normalizedDefaulted as { definition: NormalizedAgent }).definition, defaulted.program);
  const call = (req: unknown): Promise<ToolResult> => labelOf!.execute!({ req }, { callId: "t1:0:0" });
  // A `req` that fits: the default is what fails.
  const result = failed(await call({ n: 1 }));
  assert.equal(result.error.code, "evaluation-failed");
  assert.equal(result.error.diagnostics![0]!.code, "nx-ir-boundary-type");
  assert.equal("argument" in result.error.diagnostics![0]!, false);
  // A `req` that does not fit is still the model's to correct, and one with the label written works.
  const unfit = failed(await call({ n: "one" }));
  assert.equal(unfit.error.code, "invalid-input");
  assert.equal(unfit.error.diagnostics![0]!.argument, "req");
  assert.equal(succeeded(await call({ n: 1, label: "one" })).output, "one");
});

// ---- function tools -----------------------------------------------------------------------------

test("a function tool returns the function's canonical result, unwrapped", async () => {
  const result = succeeded(await run("find_plans", { teamSize: 5 }));
  assert.deepEqual(result.output, [{ $type: "Plan", name: "Team", seats: 5, monthlyPrice: 20 }]);
  assert.deepEqual(Object.keys(result), ["ok", "output", "usage"]);
});

test("a result carries what the call used", async () => {
  const sink: NxRuntimeUsage = {};
  const { callFunction } = await import("@nx-lang/ir-runtime");
  callFunction(compiled.program, { $type: "Function", module: "main.nx", name: "findPlans" }, { teamSize: 5 }, { maxOperations: 1_000_000, maxInputSize: 1_000_000, usage: sink });
  const result = succeeded(await run("find_plans", { teamSize: 5 }));
  assert.ok(sink.operations! > 0);
  // The operations of the same call made directly, and the size of the function record and the arguments.
  assert.deepEqual(result.usage, { operations: sink.operations, inputSize: sink.inputSize });
  assert.equal(result.usage!.inputSize, measureInputSize({ $type: "Function", module: "main.nx", name: "findPlans" }) + measureInputSize({ teamSize: 5 }));
});

test("input of the wrong type is invalid input, with the runtime's diagnostic and usage", async () => {
  const result = failed(await run("find_plans", { teamSize: "five" }));
  assert.equal(result.error.code, "invalid-input");
  assert.deepEqual(result.error.diagnostics, [
    { severity: "error", code: "nx-ir-boundary-type", message: "Expected teamSize to be a number.", argument: "teamSize" },
  ]);
  assert.equal(result.error.message, "Expected teamSize to be a number.");
  assert.equal("limit" in result.error, false);
  // The call reached the runtime, so it reports what it used.
  assert.equal(typeof result.usage!.operations, "number");
  assert.equal(result.usage!.inputSize, 5);
});

test("a missing required argument is invalid input, naming it", async () => {
  const result = failed(await run("find_plans", {}));
  assert.equal(result.error.code, "invalid-input");
  assert.equal(result.error.diagnostics![0]!.code, "nx-ir-arguments");
  assert.match(result.error.message, /'teamSize'/);
});

test("input that is not a JSON object is invalid input, and never reaches the runtime", async () => {
  for (const input of [null, undefined, "teamSize=5", 5, [{ teamSize: 5 }], true]) {
    const result = failed(await run("find_plans", input));
    assert.equal(result.error.code, "invalid-input", JSON.stringify(input));
    assert.match(result.error.message, /takes a JSON object/);
    assert.equal("usage" in result, false);
    assert.equal("diagnostics" in result.error, false);
  }
});

test("an argument the function does not declare is dropped", async () => {
  assert.equal(succeeded(await run("divide", { n: 5, extra: true, __proto__: { n: 1 } })).output, 2);
  assert.equal(succeeded(await run("divide", JSON.parse('{ "n": 5, "__proto__": { "polluted": true } }'))).output, 2);
  assert.equal(({} as { polluted?: boolean }).polluted, undefined);
});

test("a failure inside the function is an evaluation failure", async () => {
  const result = failed(await run("divide", { n: 0 }));
  assert.equal(result.error.code, "evaluation-failed");
  assert.equal(result.error.diagnostics![0]!.code, "nx-ir-division-by-zero");
  assert.equal(typeof result.usage!.operations, "number");
  assert.equal("limit" in result.error, false);
});

test("execute resolves and does not reject, whatever the failure", async () => {
  for (const [name, input] of [["divide", { n: 0 }], ["find_plans", { teamSize: "five" }], ["spin", { n: 1000 }], ["order_for", {}]] as const) {
    await assert.doesNotReject(run(name, input));
  }
});

// The cost of `spin`, measured: 60,810 operations for 100, 136,210 for 150 and 241,610 for 200.

test("the package's default budget applies when the host sets none", async () => {
  assert.equal(NX_AGENT_DEFAULT_MAX_OPERATIONS, 100_000);
  const within = succeeded(await run("spin", { n: 100 }));
  assert.equal(within.usage!.operations, 60_810);

  const result = failed(await run("spin", { n: 150 }));
  assert.equal(result.error.code, "resource-limit");
  assert.deepEqual(result.error.limit, { name: "maxOperations", value: 100_000 });
  assert.equal(result.error.diagnostics![0]!.code, "nx-ir-resource-limit");
  // A failure after the call began carries what it used, which is never more than the budget.
  assert.ok(result.usage!.operations! <= 100_000 && result.usage!.operations! > 90_000, String(result.usage!.operations));
  assert.equal(result.usage!.inputSize, 5);
});

test("the host's budget is used when it sets one", async () => {
  const from = toolsWith({ runtime: { maxOperations: 200_000 } });
  // Over the package's default and within the host's.
  assert.equal(succeeded(await run("spin", { n: 150 }, {}, from)).usage!.operations, 136_210);
  const result = failed(await run("spin", { n: 200 }, {}, from));
  assert.equal(result.error.code, "resource-limit");
  assert.deepEqual(result.error.limit, { name: "maxOperations", value: 200_000 });
  assert.ok(result.usage!.operations! <= 200_000);

  // A lower one is used as well: the package does not raise a budget to its default.
  const low = failed(await run("spin", { n: 100 }, {}, toolsWith({ runtime: { maxOperations: 1_000 } })));
  assert.deepEqual(low.error.limit, { name: "maxOperations", value: 1_000 });
});

test("each call gets the whole budget", async () => {
  // Three calls of 60,810 operations each, one after another: together they are over the budget,
  // and each succeeds.
  for (let call = 0; call < 3; call += 1) {
    assert.equal(succeeded(await run("spin", { n: 100 })).usage!.operations, 60_810);
  }
});

test("another limit of the runtime is a resource limit too, naming the limit", async () => {
  const result = failed(await run("spin", { n: 5 }, {}, toolsWith({ runtime: { maxRangeLength: 3 } })));
  assert.equal(result.error.code, "resource-limit");
  assert.equal(result.error.limit!.name, "maxRangeLength");
});

// ---- the two input limits -----------------------------------------------------------------------

test("input that is too large is invalid input, and the function is not called", async () => {
  assert.equal(NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE, 1_000);
  const atCap = succeeded(await run("weigh", inputOfSize(1_000), { context: chat }));
  assert.equal(atCap.output, "conv_9");

  const result = failed(await run("weigh", inputOfSize(1_001), { context: chat }));
  assert.equal(result.error.code, "invalid-input");
  assert.deepEqual(result.error.limit, { name: "maxArgumentsSize", value: 1_000 });
  assert.equal("diagnostics" in result.error, false);
  // No usage: the runtime was never called, so the function was not.
  assert.equal("usage" in result, false);
});

test("the host's context does not count against the model's cap", async () => {
  const result = succeeded(await run("weigh", inputOfSize(900), { context: contextOfSize(900) }));
  // The function record, the arguments record with the note, and the context under its name.
  assert.equal(result.usage!.inputSize, 3 + 900 + 900);
});

test("a context that is too large is a resource limit, and the function is not called", async () => {
  assert.equal(NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE, 1_000);
  succeeded(await run("weigh", inputOfSize(5), { context: contextOfSize(1_000) }));

  const result = failed(await run("weigh", inputOfSize(5), { context: contextOfSize(1_001) }));
  assert.equal(result.error.code, "resource-limit");
  assert.deepEqual(result.error.limit, { name: "maxContextSize", value: 1_000 });
  assert.equal("usage" in result, false);
});

test("the context is measured with callId set, as it is passed", async () => {
  // 1,000 with a short callId, and one more with a callId of 64 code units.
  const record = contextOfSize(1_000);
  succeeded(await run("weigh", {}, { callId: "t1:0:0", context: record }));
  const result = failed(await run("weigh", {}, { callId: "t".repeat(64), context: record }));
  assert.deepEqual(result.error.limit, { name: "maxContextSize", value: 1_000 });
});

test("a function with two context parameters is given the context twice", async () => {
  const result = succeeded(await run("twice", {}, { context: contextOfSize(900) }));
  assert.equal(result.output, "t1:0:0t1:0:0");
  assert.equal(result.usage!.inputSize, 3 + 1 + 900 + 900);
  // At both limits at once, which is exactly what the runtime's limit is set to allow.
  const atLimits = succeeded(await run("twice", inputOfSize(1_000), { context: contextOfSize(1_000) }));
  assert.equal(atLimits.usage!.inputSize, 3 + 1_000 + 1_000 + 1_000);
});

test("a context parameter with a long name is allowed what its name costs", async () => {
  assert.ok(longName.length > 64);
  assert.equal(measureInputSize({ [longName]: 0 }), 3);
  // An input at its cap and a context at its allowance: a call within both limits, which the
  // runtime's own limit must not refuse. The name costs one beyond them.
  const result = succeeded(await run("long_named", inputOfSize(1_000), { context: contextOfSize(1_000) }));
  assert.equal(result.usage!.inputSize, 3 + 1_000 + 1_000 + 1);
});

test("the host's own input limit is not used", async () => {
  const from = toolsWith({ runtime: { maxInputSize: 10 } });
  const result = succeeded(await run("weigh", inputOfSize(50), { context: chat }, from));
  assert.equal(result.output, "conv_9");
  assert.ok(result.usage!.inputSize! > 10);
});

test("a host raises and lowers the limits through the package's options", async () => {
  const raised = toolsWith({ maxArgumentsSize: 5_000, maxContextSize: 2_000 });
  succeeded(await run("weigh", inputOfSize(3_000), { context: contextOfSize(1_500) }, raised));
  assert.deepEqual(failed(await run("weigh", inputOfSize(5_001), { context: chat }, raised)).error.limit, { name: "maxArgumentsSize", value: 5_000 });

  const lowered = toolsWith({ maxArgumentsSize: 4, maxContextSize: 3 });
  succeeded(await run("weigh", inputOfSize(4), { context: contextOfSize(3) }, lowered));
  assert.deepEqual(failed(await run("weigh", inputOfSize(5), { context: contextOfSize(3) }, lowered)).error.limit, { name: "maxArgumentsSize", value: 4 });
  assert.deepEqual(failed(await run("weigh", inputOfSize(4), { context: contextOfSize(4) }, lowered)).error.limit, { name: "maxContextSize", value: 3 });
});

// ---- usage --------------------------------------------------------------------------------------

test("a failure before the call carries no usage", async () => {
  const results = [
    await run("weigh", inputOfSize(1_001), { context: chat }),
    await run("weigh", "not an object", { context: chat }),
    await run("weigh", {}, { context: chat, signal: AbortSignal.abort() }),
    await run("weigh", {}),
    await run("weigh", {}, { context: contextOfSize(1_001) }),
  ];
  assert.deepEqual(
    results.map((result) => [failed(result).error.code, "usage" in result]),
    [
      ["invalid-input", false],
      ["invalid-input", false],
      ["aborted", false],
      ["invalid-context", false],
      ["resource-limit", false],
    ],
  );
});

test("the host's own usage object is left alone", async () => {
  const hostUsage: NxRuntimeUsage = { operations: 7, inputSize: 9 };
  const from = toolsWith({ runtime: { usage: hostUsage } });
  const result = succeeded(await run("find_plans", { teamSize: 5 }, {}, from));
  assert.ok(result.usage!.operations! > 0);
  assert.deepEqual(hostUsage, { operations: 7, inputSize: 9 });
  failed(await run("spin", { n: 1000 }, {}, from));
  assert.deepEqual(hostUsage, { operations: 7, inputSize: 9 });
});

test("calls started together carry their own numbers", async () => {
  const hostUsage: NxRuntimeUsage = {};
  const from = toolsWith({ runtime: { usage: hostUsage } });
  const [small, large, failing] = await Promise.all([
    run("find_plans", { teamSize: 5 }, {}, from),
    run("spin", { n: 100 }, {}, from),
    run("spin", { n: 1000 }, {}, from),
  ]);
  assert.equal(succeeded(large!).usage!.operations, 60_810);
  assert.ok(succeeded(small!).usage!.operations! < 100);
  assert.ok(failed(failing!).usage!.operations! > 90_000);
  assert.notEqual(small!.usage, large!.usage);
  assert.deepEqual(hostUsage, {});
});

// ---- host executors -----------------------------------------------------------------------------

test("a host kind is executed with the stored config, the input and the call context", async () => {
  const seen: unknown[] = [];
  const signal = new AbortController().signal;
  const from = toolsWith({
    executors: {
      builtin: async (tool, input, context) => {
        seen.push({ tool, input, context });
        return { records: [{ name: "Acme" }] };
      },
    },
  });
  const result = succeeded(await run("search_records", { query: "acme" }, { callId: "t3:0:1", context: chat, signal }, from));
  assert.deepEqual(result, { ok: true, output: { records: [{ name: "Acme" }] } });
  assert.equal("usage" in result, false);
  assert.equal(seen.length, 1);
  const call = seen[0] as { tool: NormalizedTool; input: unknown; context: ToolCallContext };
  assert.deepEqual(call.tool.config, { builtin: "record_search", recordKind: "company" });
  assert.equal(call.tool.name, "search_records");
  assert.deepEqual(call.input, { query: "acme" });
  assert.equal(call.context.callId, "t3:0:1");
  assert.equal(call.context.signal, signal);
  assert.deepEqual(call.context.context, chat);
});

test("a host executor may answer without a promise, and its input is passed as it came", async () => {
  const from = toolsWith({ executors: { builtin: (_tool, input) => (input === "raw" ? "saw raw" : "other") } });
  assert.equal(succeeded(await run("search_records", "raw", {}, from)).output, "saw raw");
});

test("a host executor failure is a result, with the code of the package's tool error when it threw one", async () => {
  const throwing = toolsWith({
    executors: {
      builtin: async () => {
        throw new Error("the record index is down");
      },
    },
  });
  const plain = failed(await run("search_records", { query: "x" }, {}, throwing));
  assert.deepEqual(plain, { ok: false, error: { code: "evaluation-failed", message: "the record index is down" } });

  const coded = toolsWith({
    executors: {
      builtin: () => {
        throw new NxAgentToolError("rate-limited", "Too many searches.", { limit: { name: "searchesPerMinute", value: 10 } });
      },
    },
  });
  assert.deepEqual(failed(await run("search_records", { query: "x" }, {}, coded)), {
    ok: false,
    error: { code: "rate-limited", message: "Too many searches.", limit: { name: "searchesPerMinute", value: 10 } },
  });
  await assert.doesNotReject(run("search_records", { query: "x" }, {}, throwing));
});
