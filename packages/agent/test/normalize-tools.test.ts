import assert from "node:assert/strict";
import { after, test } from "node:test";

import type { Agent, NormalizedAgent, NormalizedTool, NxAgentDiagnostic } from "../src/index.js";
import { normalizeAgent, type AgentFunctionSchema, type AgentSchemaSource, type NormalizeAgentOptions, type NormalizeAgentResult } from "../src/normalize.js";
import { chatToolContext, compileDiagnostics, compileProgram, disposeCompiled, type CompiledProgram } from "./nx-support.js";

after(disposeCompiled);

const SCHEMA = "https://json-schema.org/draft/2020-12/schema";
const toolContext = { module: "@nx/agent/agent.nx", name: "ToolContext" } as const;

/** A program whose `root` is an agent with `tools`, over `declarations`. */
function agentProgram(declarations: string, tools: string): CompiledProgram {
  return compileProgram(`${declarations}\nlet root(): Agent = { <Agent name="support" tools={ ${tools} }>Be brief.</Agent> }`);
}

function normalize(compiled: CompiledProgram, options: Omit<NormalizeAgentOptions, "schemas"> = {}, change?: (agent: Agent) => Agent): NormalizeAgentResult {
  const agent = compiled.agent();
  return normalizeAgent(change === undefined ? agent : change(agent), { schemas: compiled.artifact, ...options });
}

/** A value built by hand, as one that did not come from evaluating a checked program is. */
function handBuilt(agent: Agent, tools: readonly unknown[]): Agent {
  return { ...agent, tools } as unknown as Agent;
}

function definitionOf(result: NormalizeAgentResult): NormalizedAgent {
  assert.equal(result.ok, true, JSON.stringify(result.diagnostics, null, 2));
  return (result as Extract<NormalizeAgentResult, { ok: true }>).definition;
}

function onlyTool(result: NormalizeAgentResult): NormalizedTool {
  const { tools } = definitionOf(result);
  assert.equal(tools.length, 1);
  return tools[0]!;
}

/** The diagnostics of a result with no definition. */
function refused(result: NormalizeAgentResult): readonly NxAgentDiagnostic[] {
  assert.equal(result.ok, false, "the agent normalized");
  assert.equal("definition" in result, false);
  assert.ok(result.diagnostics.some((diagnostic) => diagnostic.severity === "error"));
  return result.diagnostics;
}

function onlyDiagnostic(result: NormalizeAgentResult): NxAgentDiagnostic {
  const diagnostics = refused(result);
  assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics, null, 2));
  return diagnostics[0]!;
}

const plans = `
/// A plan a team can buy.
type Plan = { name:string seats:int monthlyPrice:int }

/// Finds the plans that fit a team.
let findPlans(
  /// Number of people who need a seat.
  teamSize: int,
  maxMonthlyPrice?: int
): Plan* = { <Plan name="Team" seats={teamSize} monthlyPrice=20 /> }
`;

// ---- FunctionTool -------------------------------------------------------------------------------

test("a function tool takes its name, description and schemas from its function", () => {
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  const tool = onlyTool(normalize(compiled));
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: "findPlans" });
  assert.equal(tool.name, "find_plans");
  assert.equal(tool.description, "Finds the plans that fit a team.");
  assert.equal(tool.kind, "function");
  assert.deepEqual(tool.config, { function: { module: "main.nx", name: "findPlans" }, contextParameters: [] });
  assert.deepEqual(tool.annotations, { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false });
  // The schemas are the export's, unchanged.
  assert.deepEqual(tool.inputSchema, answer.inputSchema);
  assert.deepEqual(tool.outputSchema, answer.outputSchema);
  assert.deepEqual(tool.inputSchema, {
    $schema: SCHEMA,
    type: "object",
    properties: {
      teamSize: { type: "integer", description: "Number of people who need a seat." },
      maxMonthlyPrice: { type: "integer" },
    },
    required: ["teamSize"],
    additionalProperties: false,
  });
  assert.deepEqual(Object.keys(tool.inputSchema["properties"] as object), ["teamSize", "maxMonthlyPrice"]);
  // The MCP fields, in a fixed order, with no title and nothing executable.
  assert.deepEqual(Object.keys(tool), ["name", "description", "inputSchema", "outputSchema", "annotations", "kind", "config"]);
  assert.equal("execute" in tool, false);
});

test("an array result is not wrapped", () => {
  const tool = onlyTool(normalize(agentProgram(plans, "<FunctionTool function={findPlans} />")));
  assert.equal(tool.outputSchema!["type"], "array");
  assert.deepEqual(tool.outputSchema!["items"], { $ref: "#/$defs/Plan" });
});

test("the definition shares nothing with the schemas it was made from", () => {
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  const held: AgentFunctionSchema[] = [];
  const source: AgentSchemaSource = {
    functionSchema: (reference, options) => {
      const answer = compiled.artifact.functionSchema(reference, options);
      held.push(answer);
      return answer;
    },
  };
  const tool = onlyTool(normalizeAgent(compiled.agent(), { schemas: source }));
  assert.equal(held.length, 1);
  assert.notEqual(tool.inputSchema, held[0]!.inputSchema);
  assert.deepEqual(tool.inputSchema, held[0]!.inputSchema);
});

test("a parameter with a default, an optional parameter and no parameters", () => {
  const compiled = agentProgram(
    `
/// Lists orders.
let listOrders(state: string, limit: int = 10, after?: string): string* = { state }
/// Answers pong.
let ping(): string = { "pong" }`,
    "<FunctionTool function={listOrders} /> <FunctionTool function={ping} />",
  );
  const [listOrders, ping] = definitionOf(normalize(compiled)).tools;
  assert.deepEqual(Object.keys(listOrders!.inputSchema["properties"] as object), ["state", "limit", "after"]);
  assert.deepEqual(listOrders!.inputSchema["required"], ["state"]);
  assert.deepEqual((listOrders!.inputSchema["properties"] as Record<string, unknown>)["limit"], { type: "integer", default: 10 });
  assert.equal(listOrders!.inputSchema["additionalProperties"], false);
  assert.deepEqual(ping!.inputSchema, { $schema: SCHEMA, type: "object", properties: {}, additionalProperties: false });
  assert.deepEqual(ping!.outputSchema, { $schema: SCHEMA, type: "string" });
});

test("acronyms and element-style names are converted", () => {
  const compiled = agentProgram(
    `
/// One.
let HTTPStatusFor(code:int): string = { "ok" }
/// Two.
let PlanRow(name:string): string = { name }
/// Three.
let lookupOrder2(id:string): string = { id }`,
    "<FunctionTool function={HTTPStatusFor} /> <FunctionTool function={PlanRow} /> <FunctionTool function={lookupOrder2} />",
  );
  assert.deepEqual(
    definitionOf(normalize(compiled)).tools.map((tool) => tool.name),
    ["http_status_for", "plan_row", "lookup_order2"],
  );
});

test("an authored name and description win over the function's", () => {
  const tool = onlyTool(normalize(agentProgram(plans, '<FunctionTool function={findPlans} name="plans" description="Search plans." />')));
  assert.equal(tool.name, "plans");
  assert.equal(tool.description, "Search plans.");
});

test("an authored name that is not snake_case is rejected, not converted", () => {
  const diagnostic = onlyDiagnostic(normalize(agentProgram(plans, '<FunctionTool function={findPlans} name="Find Plans" />')));
  assert.equal(diagnostic.code, "nx-agent-invalid-tool-name");
  assert.equal(diagnostic.path, "tools[0].name");
  assert.match(diagnostic.message, /"Find Plans"/);
  assert.ok(diagnostic.message.includes("^[a-z][a-z0-9_]{0,63}$"));
  assert.equal("tool" in diagnostic, false);
});

test("a derived name that does not fit the pattern is rejected, pointing at the function", () => {
  const long = `f${"x".repeat(70)}`;
  const source = `/// Long.\nlet ${long}(n:int): int = { n }`;
  const compiled = agentProgram(source, `<FunctionTool function={${long}} />`);
  const diagnostic = onlyDiagnostic(normalize(compiled));
  assert.equal(diagnostic.code, "nx-agent-invalid-tool-name");
  assert.equal(diagnostic.path, "tools[0]");
  assert.equal("tool" in diagnostic, false);
  assert.match(diagnostic.message, /defaults to "fx+", which does not match/);
  // The span is the export's own, which begins at the function's `let`, after its doc comment.
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: long });
  assert.deepEqual(diagnostic.declaration, { module: "main.nx", span: answer.declaration });
  assert.ok(source.split("\n")[diagnostic.declaration!.span!.startLine - 1]!.startsWith(`let ${long}(`));
});

test("a tool with no description is rejected, naming the tool and pointing at its function", () => {
  const compiled = agentProgram(`${plans}\nlet undocumented(n:int): int = { n }`, "<FunctionTool function={findPlans} /> <FunctionTool function={undocumented} />");
  const diagnostic = onlyDiagnostic(normalize(compiled));
  assert.equal(diagnostic.code, "nx-agent-missing-description");
  assert.equal(diagnostic.path, "tools[1]");
  assert.equal(diagnostic.tool, "undocumented");
  assert.match(diagnostic.message, /Tool 'undocumented' \(tools\[1\]\) has no description/);
  // The span the export answered for the function's declaration, copied whole.
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: "undocumented" });
  assert.ok(answer.declaration !== undefined);
  assert.deepEqual(diagnostic.declaration, { module: "main.nx", span: answer.declaration });
});

test("an authored description of white space is rejected, and is not replaced by the doc comment", () => {
  const diagnostic = onlyDiagnostic(normalize(agentProgram(plans, '<FunctionTool function={findPlans} description="   " />')));
  assert.equal(diagnostic.code, "nx-agent-missing-description");
  assert.equal(diagnostic.path, "tools[0].description");
  assert.equal(diagnostic.tool, "find_plans");
  assert.match(diagnostic.message, /has an empty description/);
});

test("a function-typed parameter is rejected, keeping the export's message", () => {
  const compiled = agentProgram(
    `
/// Applies a function.
let applyTo(f: <function n:int />: int, n:int): int = <f n={n} />`,
    "<FunctionTool function={applyTo} />",
  );
  const diagnostic = onlyDiagnostic(normalize(compiled));
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: "applyTo" });
  assert.equal(answer.inputSchema, undefined);
  assert.equal(diagnostic.code, "nx-agent-inexpressible-parameter");
  assert.equal(diagnostic.severity, "error");
  assert.equal(diagnostic.path, "tools[0]");
  assert.equal(diagnostic.tool, "apply_to");
  assert.equal(diagnostic.message, `Tool 'apply_to' (tools[0]): ${answer.diagnostics[0]!.message}`);
  assert.match(diagnostic.message, /Parameter `f` has the function type `<function n:int \/>: int`/);
  assert.deepEqual(diagnostic.declaration, { module: "main.nx", span: answer.declaration });
});

test("a result with no JSON form is a warning, and the tool has no output schema", () => {
  const compiled = agentProgram(
    `
/// Triples a number.
let triple(n:int): int = { n * 3 }
/// Picks a function.
let pick(n:int): <function n:int />: int = { triple }`,
    "<FunctionTool function={pick} />",
  );
  const result = normalize(compiled);
  const tool = onlyTool(result);
  assert.equal("outputSchema" in tool, false);
  assert.deepEqual(Object.keys(tool), ["name", "description", "inputSchema", "annotations", "kind", "config"]);
  assert.equal(result.diagnostics.length, 1);
  const warning = result.diagnostics[0]!;
  assert.equal(warning.severity, "warning");
  assert.equal(warning.code, "nx-agent-inexpressible-result");
  assert.equal(warning.path, "tools[0]");
  assert.equal(warning.tool, "pick");
  assert.match(warning.message, /Tool 'pick' \(tools\[0\]\) has no output schema: The result of `pick` has the function type/);
  assert.equal(warning.declaration?.module, "main.nx");
});

test("a function with neither schema is refused, each of the export's diagnostics an error", () => {
  const compiled = agentProgram(
    `
/// Passes a function through.
let both(f: <function n:int />: int): <function n:int />: int = { f }`,
    "<FunctionTool function={both} />",
  );
  const diagnostics = refused(normalize(compiled));
  // The export does not say which diagnostic is about what, so with no input schema each is an
  // error under the parameter's code, the result's among them. Its message says what it is about.
  assert.deepEqual(
    diagnostics.map((diagnostic) => [diagnostic.severity, diagnostic.code]),
    [
      ["error", "nx-agent-inexpressible-parameter"],
      ["error", "nx-agent-inexpressible-parameter"],
    ],
  );
  assert.match(diagnostics[0]!.message, /Parameter `f`/);
  assert.match(diagnostics[1]!.message, /The result of `both`/);
});

test("a function the program does not declare is rejected, naming the module and the function", () => {
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  const stale = (agent: Agent): Agent => handBuilt(agent, [{ $type: "FunctionTool", function: { $type: "Function", module: "main.nx", name: "findPlanz" } }]);
  const diagnostic = onlyDiagnostic(normalize(compiled, {}, stale));
  assert.equal(diagnostic.code, "nx-agent-unknown-function");
  assert.equal(diagnostic.path, "tools[0].function");
  assert.match(diagnostic.message, /'findPlanz' of module 'main\.nx'/);
  // There is no answer to copy a span from, so the module alone.
  assert.deepEqual(diagnostic.declaration, { module: "main.nx" });

  const otherModule = (agent: Agent): Agent => handBuilt(agent, [{ $type: "FunctionTool", function: { $type: "Function", module: "other.nx", name: "findPlans" } }]);
  assert.match(onlyDiagnostic(normalize(compiled, {}, otherModule)).message, /'findPlans' of module 'other\.nx'/);
});

test("a function field that is not a function record is rejected", () => {
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  for (const held of [undefined, "findPlans", { module: "main.nx", name: "findPlans" }, { $type: "Function", name: "findPlans" }]) {
    const diagnostic = onlyDiagnostic(normalize(compiled, {}, (agent) => handBuilt(agent, [{ $type: "FunctionTool", function: held }])));
    assert.equal(diagnostic.code, "nx-agent-unknown-function");
    assert.equal(diagnostic.path, "tools[0].function");
  }
});

test("anything else a schema source throws is thrown on", () => {
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  const disposed = new Error("The artifact has been disposed.");
  const source: AgentSchemaSource = {
    functionSchema: () => {
      throw disposed;
    },
  };
  assert.throws(() => normalizeAgent(compiled.agent(), { schemas: source }), (error) => error === disposed);
  // An error with diagnostics of another code is not the unknown-function report either.
  const other = Object.assign(new Error("crashed"), { diagnostics: [{ code: "host-crashed", message: "crashed" }] });
  assert.throws(
    () =>
      normalizeAgent(compiled.agent(), {
        schemas: {
          functionSchema: () => {
            throw other;
          },
        },
      }),
    (error) => error === other,
  );
});

test("the span is left out only when the export leaves it out", () => {
  const compiled = agentProgram(`${plans}\nlet undocumented(n:int): int = { n }`, "<FunctionTool function={undocumented} />");
  const withoutSpan: AgentSchemaSource = {
    functionSchema: (reference, options) => {
      const { declaration: _declaration, ...answer } = compiled.artifact.functionSchema(reference, options);
      return answer;
    },
  };
  const diagnostic = onlyDiagnostic(normalizeAgent(compiled.agent(), { schemas: withoutSpan }));
  assert.equal(diagnostic.code, "nx-agent-missing-description");
  assert.deepEqual(diagnostic.declaration, { module: "main.nx" });
});

// ---- context parameters -------------------------------------------------------------------------

const contextFunctions = `
/// Looks an order up for the conversation.
let orderFor(orderId: string, context: ChatToolContext): string = { context.conversationId }
/// Says which call this is.
let whoAmI(context: ToolContext): string = { context.callId }
`;

test("a context parameter is left out of the schema and recorded with its declared type", () => {
  const compiled = agentProgram(contextFunctions, "<FunctionTool function={orderFor} /> <FunctionTool function={whoAmI} />");
  const [orderFor, whoAmI] = definitionOf(normalize(compiled, { toolContextType: chatToolContext })).tools;
  assert.deepEqual(Object.keys(orderFor!.inputSchema["properties"] as object), ["orderId"]);
  assert.deepEqual(orderFor!.inputSchema["required"], ["orderId"]);
  assert.deepEqual(orderFor!.config, {
    function: { module: "main.nx", name: "orderFor" },
    contextParameters: [{ name: "context", type: { module: "host/Chat.nx", name: "ChatToolContext" } }],
  });
  // A function with only a context parameter takes no input.
  assert.deepEqual(whoAmI!.inputSchema, { $schema: SCHEMA, type: "object", properties: {}, additionalProperties: false });
  assert.deepEqual(whoAmI!.config, {
    function: { module: "main.nx", name: "whoAmI" },
    contextParameters: [{ name: "context", type: toolContext }],
  });
});

test("a context parameter the host cannot fill is rejected: no context type was named", () => {
  const compiled = agentProgram(contextFunctions, "<FunctionTool function={orderFor} /> <FunctionTool function={whoAmI} />");
  const diagnostics = refused(normalize(compiled));
  assert.deepEqual(
    diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.path, diagnostic.tool]),
    [
      ["nx-agent-context-parameter", "tools[0]", "order_for"],
      ["nx-agent-context-parameter", "tools[1]", "who_am_i"],
    ],
  );
  assert.match(diagnostics[0]!.message, /the parameter 'context' of 'orderFor' is a tool context, and the host named no tool context type/);
  assert.equal(diagnostics[0]!.declaration?.module, "main.nx");
  assert.ok(diagnostics[0]!.declaration?.span !== undefined);
});

test("a context parameter the host cannot fill is rejected: it is declared with another subtype", () => {
  const compiled = agentProgram(contextFunctions, "<FunctionTool function={orderFor} /> <FunctionTool function={whoAmI} />");
  const result = normalize(compiled, { toolContextType: { module: "host/Other.nx", name: "OtherToolContext" } });
  // `whoAmI` declares the base type, which any host's record satisfies, so only `orderFor` fails.
  const diagnostic = onlyDiagnostic(result);
  assert.equal(diagnostic.code, "nx-agent-context-parameter");
  assert.equal(diagnostic.tool, "order_for");
  assert.match(diagnostic.message, /'context' of 'orderFor' is declared with 'ChatToolContext' of 'host\/Chat\.nx', and the host supplies 'OtherToolContext' of 'host\/Other\.nx'/);
});

test("types are compared by module as well as name", () => {
  const compiled = agentProgram(contextFunctions, "<FunctionTool function={orderFor} />");
  const sameNameElsewhere = { module: "elsewhere/Chat.nx", name: "ChatToolContext" };
  assert.equal(onlyDiagnostic(normalize(compiled, { toolContextType: sameNameElsewhere })).code, "nx-agent-context-parameter");
});

/** The one diagnostic of a function tool over `declarations`, normalized with the host's context type named. */
function contextDiagnostic(declarations: string, name: string): { diagnostic: NxAgentDiagnostic; compiled: CompiledProgram } {
  const compiled = agentProgram(declarations, `<FunctionTool function={${name}} />`);
  return { diagnostic: onlyDiagnostic(normalize(compiled, { toolContextType: chatToolContext })), compiled };
}

test("a sequence of contexts is rejected", () => {
  const { diagnostic, compiled } = contextDiagnostic(
    `
/// Counts the contexts it is given.
let countContexts(orderId: string, contexts: ChatToolContext+): int = { 1 }`,
    "countContexts",
  );
  assert.equal(diagnostic.code, "nx-agent-context-parameter");
  assert.equal(diagnostic.severity, "error");
  assert.equal(diagnostic.path, "tools[0]");
  assert.equal(diagnostic.tool, "count_contexts");
  assert.match(diagnostic.message, /Tool 'count_contexts' \(tools\[0\]\): the parameter 'contexts' of 'countContexts' holds a tool context without being one, so the model would be asked for it/);
  // What would have gone to the model: the export keeps the parameter, context record and all.
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: "countContexts" }, { hostSuppliedTypes: [toolContext] });
  assert.deepEqual(Object.keys(answer.inputSchema!["properties"] as object), ["orderId", "contexts"]);
  assert.deepEqual(answer.parameters[1]!.hostSuppliedWithin, [toolContext]);
  assert.deepEqual(diagnostic.declaration, { module: "main.nx", span: answer.declaration });
});

test("a context held in another record is rejected, at any depth", () => {
  const records = `
type Request = { orderId:string context:ChatToolContext }
type Batch = { requests:Request+ }`;
  const held = contextDiagnostic(`${records}\n/// Sends one request.\nlet sendOne(request: Request): string = { request.orderId }`, "sendOne");
  assert.equal(held.diagnostic.code, "nx-agent-context-parameter");
  assert.equal(held.diagnostic.tool, "send_one");
  assert.match(held.diagnostic.message, /the parameter 'request' of 'sendOne' holds a tool context/);

  const deeper = contextDiagnostic(`${records}\n/// Sends a batch.\nlet sendAll(note: string, batch?: Batch): string = { note }`, "sendAll");
  assert.match(deeper.diagnostic.message, /the parameter 'batch' of 'sendAll' holds a tool context/);
});

test("a context of a subtype the host did not name is still found", () => {
  // The host supplies `ChatToolContext`. `AuditToolContext` is another context type, which the
  // package has never been told of; the compiler knows it extends `ToolContext`.
  const { diagnostic } = contextDiagnostic("/// Audits.\nlet audit(contexts: AuditToolContext+): int = { 1 }", "audit");
  assert.equal(diagnostic.code, "nx-agent-context-parameter");
  assert.match(diagnostic.message, /the parameter 'contexts' of 'audit' holds a tool context/);

  const base = contextDiagnostic("/// Any.\nlet anyOf(orderId: string, contexts?: ToolContext+): int = { 1 }", "anyOf");
  assert.match(base.diagnostic.message, /the parameter 'contexts' of 'anyOf' holds a tool context/);
});

test("a context parameter declared through an alias is a context parameter", () => {
  const compiled = agentProgram(
    `
type Context = ChatToolContext
type Contexts = ChatToolContext+
/// Looks an order up for the conversation.
let viaAlias(orderId: string, context: Context): string = { orderId + "@" + context.conversationId }
/// Counts contexts.
let viaListAlias(contexts: Contexts): int = { 1 }`,
    "<FunctionTool function={viaAlias} />",
  );
  const tool = onlyTool(normalize(compiled, { toolContextType: chatToolContext }));
  assert.deepEqual(Object.keys(tool.inputSchema["properties"] as object), ["orderId"]);
  // The type recorded is the one the alias denotes, which is the host's.
  assert.deepEqual(tool.config, {
    function: { module: "main.nx", name: "viaAlias" },
    contextParameters: [{ name: "context", type: chatToolContext }],
  });
  // An alias of a list of contexts is a list of contexts.
  const list = (agent: Agent): Agent => handBuilt(agent, [{ $type: "FunctionTool", function: { $type: "Function", module: "main.nx", name: "viaListAlias" } }]);
  assert.match(onlyDiagnostic(normalize(compiled, { toolContextType: chatToolContext }, list)).message, /'contexts' of 'viaListAlias' holds a tool context/);
});

test("a context that is a parameter of its own is accepted beside one that is held", () => {
  const compiled = agentProgram(
    `
type Request = { context:ChatToolContext }
/// Takes the context twice, once the right way.
let mixed(context: ChatToolContext, request: Request): string = { context.conversationId }`,
    "<FunctionTool function={mixed} />",
  );
  // Only the held one is reported; with no host type named, both are.
  assert.match(onlyDiagnostic(normalize(compiled, { toolContextType: chatToolContext })).message, /'request' of 'mixed' holds a tool context/);
  assert.deepEqual(
    refused(normalize(compiled)).map((diagnostic) => diagnostic.message.match(/the parameter '(\w+)'/)![1]),
    ["context", "request"],
  );
});

test("a schema source that answers from what it kept is held to the same rule", () => {
  const compiled = agentProgram(contextFunctions, "<FunctionTool function={orderFor} />");
  const answering = (held: readonly { module: string; name: string }[]): AgentSchemaSource => ({
    functionSchema: (reference, options) => {
      const answer = compiled.artifact.functionSchema(reference, options);
      return { ...answer, parameters: [{ name: "orderId", hostSuppliedWithin: held }, answer.parameters[1]!] };
    },
  });
  const diagnostic = onlyDiagnostic(normalizeAgent(compiled.agent(), { schemas: answering([toolContext]), toolContextType: chatToolContext }));
  assert.equal(diagnostic.code, "nx-agent-context-parameter");
  assert.match(diagnostic.message, /'orderId' of 'orderFor' holds a tool context/);
  // An empty list is none.
  assert.equal(normalizeAgent(compiled.agent(), { schemas: answering([]), toolContextType: chatToolContext }).ok, true);
});

test("a schema keyword the package does not know is kept", () => {
  // What a constrained type of a later NX would write, and a keyword that does not exist.
  const compiled = agentProgram(plans, "<FunctionTool function={findPlans} />");
  const inputSchema = {
    $schema: SCHEMA,
    type: "object",
    properties: {
      teamSize: { type: "integer", minimum: 1, maximum: 10, "x-nx-constraint": "1..10" },
      code: { type: "string", minLength: 2, maxLength: 8, pattern: "^[A-Z]+$" },
    },
    required: ["teamSize"],
    additionalProperties: false,
  };
  const outputSchema = { $schema: SCHEMA, type: "array", items: { type: "string", pattern: "^plan_" }, maxItems: 5, futureKeyword: { nested: [1, 2] } };
  const source: AgentSchemaSource = {
    functionSchema: (reference, options) => ({ ...compiled.artifact.functionSchema(reference, options), inputSchema, outputSchema }),
  };
  const tool = onlyTool(normalizeAgent(compiled.agent(), { schemas: source }));
  assert.deepEqual(tool.inputSchema, inputSchema);
  assert.deepEqual(tool.outputSchema, outputSchema);
  assert.equal(JSON.stringify(tool.inputSchema), JSON.stringify(inputSchema));
  assert.equal(JSON.stringify(tool.outputSchema), JSON.stringify(outputSchema));
});

// ---- WebSearchTool ------------------------------------------------------------------------------

test("a web search tool is described as a provider tool with its domains", () => {
  const tool = onlyTool(normalize(agentProgram("", '<WebSearchTool allowedDomains={ "docs.example.com" } />')));
  assert.deepEqual(tool, {
    name: "web_search",
    description: "Searches the web and returns relevant results.",
    inputSchema: { $schema: SCHEMA, type: "object", properties: {}, additionalProperties: false },
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: true },
    kind: "provider",
    config: { provider: "web_search", allowedDomains: ["docs.example.com"] },
  });
  assert.equal("outputSchema" in tool, false);
  assert.equal("title" in tool, false);
});

test("a web search tool with no domains has none in its config, and takes an authored name and description", () => {
  const tool = onlyTool(normalize(agentProgram("", '<WebSearchTool name="search_docs" description="Search the docs." />')));
  assert.equal(tool.name, "search_docs");
  assert.equal(tool.description, "Search the docs.");
  assert.deepEqual(tool.config, { provider: "web_search" });
});

// ---- HttpTool -----------------------------------------------------------------------------------

const orders = `
/// A new order, sent as a request body.
type NewOrder = { sku:string quantity:int }

/// Looks an order up by its identifier.
let lookupOrder(orderId: string): HttpArguments = {
  <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> />
}

/// Looks an order up for the conversation.
let lookupMine(orderId: string, context: ChatToolContext): HttpArguments = {
  <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> />
}
`;

function httpProgram(baseUrl: string, tool: string): CompiledProgram {
  return agentProgram(`${orders}\nlet shop = <HttpConnection name="shop" baseUrl="${baseUrl}" />`, tool);
}

const getOrder = '<HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />';

test("an HTTP tool normalizes to a fixed operation on its connection", () => {
  const compiled = httpProgram("https://api.example.com", getOrder);
  const tool = onlyTool(normalize(compiled));
  const answer = compiled.artifact.functionSchema({ module: "main.nx", name: "lookupOrder" });
  assert.equal(tool.name, "lookup_order");
  assert.equal(tool.description, "Looks an order up by its identifier.");
  assert.equal(tool.kind, "http");
  assert.deepEqual(tool.inputSchema, answer.inputSchema);
  assert.deepEqual(tool.config, {
    arguments: { module: "main.nx", name: "lookupOrder" },
    contextParameters: [],
    connection: { name: "shop", baseUrl: "https://api.example.com" },
    method: "get",
    path: "/orders/{orderId}",
    pathPlaceholders: ["orderId"],
  });
  assert.deepEqual(Object.keys(tool.config), ["arguments", "contextParameters", "connection", "method", "path", "pathPlaceholders"]);
  assert.deepEqual(tool.annotations, { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: true });
  assert.equal("title" in tool, false);
});

test("an HTTP tool's output is an open object with a required integer status and a body of any type", () => {
  const tool = onlyTool(normalize(httpProgram("https://api.example.com", getOrder)));
  assert.deepEqual(tool.outputSchema, {
    $schema: SCHEMA,
    type: "object",
    properties: { status: { type: "integer" }, body: {} },
    required: ["status"],
  });
  // The object is open: nothing forbids a property the host's request function adds.
  assert.equal("additionalProperties" in tool.outputSchema!, false);
});

test("a base URL's trailing slash is removed, and its path kept", () => {
  const tool = onlyTool(normalize(httpProgram("https://api.example.com/v1/", getOrder)));
  assert.deepEqual((tool.config as { connection: unknown }).connection, { name: "shop", baseUrl: "https://api.example.com/v1" });
});

test("each method has its own annotations, and the authored call limit is kept", () => {
  const tools = ["get", "post", "put", "patch", "delete"]
    .map((method) => `<HttpTool name="${method}_order" connection={shop} method={HttpMethod.${method}} path="/orders/{orderId}" arguments={lookupOrder} maxCallsPerConversation=3 />`)
    .join(" ");
  const definition = definitionOf(normalize(httpProgram("https://api.example.com", tools)));
  assert.deepEqual(
    definition.tools.map((tool) => [(tool.config as { method: string }).method, tool.annotations.readOnlyHint, tool.annotations.destructiveHint, tool.annotations.idempotentHint, tool.annotations.openWorldHint]),
    [
      ["get", true, false, true, true],
      ["post", false, true, false, true],
      ["put", false, true, true, true],
      ["patch", false, true, false, true],
      ["delete", false, true, true, true],
    ],
  );
  assert.equal((definition.tools[1]!.config as { maxCallsPerConversation?: number }).maxCallsPerConversation, 3);
  assert.equal(Object.keys(definition.tools[1]!.config).at(-1), "maxCallsPerConversation");
});

test("an HTTP tool's context parameters are recorded as a function tool's are", () => {
  const tool = '<HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupMine} />';
  const normalized = onlyTool(normalize(httpProgram("https://api.example.com", tool), { toolContextType: chatToolContext }));
  assert.deepEqual(Object.keys(normalized.inputSchema["properties"] as object), ["orderId"]);
  assert.deepEqual((normalized.config as { contextParameters: unknown }).contextParameters, [{ name: "context", type: chatToolContext }]);
  assert.equal(onlyDiagnostic(normalize(httpProgram("https://api.example.com", tool))).code, "nx-agent-context-parameter");
});

test("a base URL that is not https is rejected, naming the connection and the scheme", () => {
  const diagnostic = onlyDiagnostic(normalize(httpProgram("http://api.example.com", getOrder)));
  assert.equal(diagnostic.code, "nx-agent-http-base-url");
  assert.equal(diagnostic.path, "tools[0].connection.baseUrl");
  assert.equal(diagnostic.tool, "lookup_order");
  assert.match(diagnostic.message, /the base URL 'http:\/\/api\.example\.com' of connection 'shop' cannot be used: its scheme is 'http', not 'https'/);
});

test("the host can allow http for a local server, and nothing else", () => {
  const local = httpProgram("http://localhost:8787", getOrder);
  assert.equal(onlyDiagnostic(normalize(local)).code, "nx-agent-http-base-url");
  assert.equal(onlyDiagnostic(normalize(local, { allowInsecureBaseUrl: false })).code, "nx-agent-http-base-url");
  const tool = onlyTool(normalize(local, { allowInsecureBaseUrl: true }));
  assert.deepEqual((tool.config as { connection: unknown }).connection, { name: "shop", baseUrl: "http://localhost:8787" });
  assert.equal(onlyDiagnostic(normalize(httpProgram("ftp://api.example.com", getOrder), { allowInsecureBaseUrl: true })).code, "nx-agent-http-base-url");
});

test("a base URL with a query, a fragment, user information or no scheme is rejected, naming the connection", () => {
  const cases: readonly (readonly [string, RegExp])[] = [
    ["https://api.example.com/v1?key=1", /has a query/],
    ["https://api.example.com/v1#top", /has a fragment/],
    ["https://user:secret@api.example.com", /has user information/],
    ["api.example.com", /not an absolute URL/],
    ["https://API.example.com", /a URL parser reads it as 'https:\/\/api\.example\.com'; write it that way/],
  ];
  for (const [baseUrl, problem] of cases) {
    const diagnostic = onlyDiagnostic(normalize(httpProgram(baseUrl, getOrder)));
    assert.equal(diagnostic.code, "nx-agent-http-base-url", baseUrl);
    assert.match(diagnostic.message, /connection 'shop'/, baseUrl);
    assert.match(diagnostic.message, problem, baseUrl);
  }
});

test("a malformed path is rejected, naming the tool and the path", () => {
  const cases: readonly (readonly [string, RegExp])[] = [
    ["/orders/../admin", /has the segment '\.\.'/],
    ["orders/{orderId}", /does not begin with '\/'/],
    ["/orders/{orderId", /is not closed/],
    ["/orders/{order-id}", /not named by an identifier/],
    ["/orders?id={orderId}", /is not a URL path character/],
    ["/my orders/{orderId}", /is not a URL path character/],
  ];
  for (const [path, problem] of cases) {
    const tool = `<HttpTool connection={shop} method={HttpMethod.get} path="${path}" arguments={lookupOrder} />`;
    const diagnostic = onlyDiagnostic(normalize(httpProgram("https://api.example.com", tool)));
    assert.equal(diagnostic.code, "nx-agent-http-path", path);
    assert.equal(diagnostic.path, "tools[0].path", path);
    assert.equal(diagnostic.tool, "lookup_order", path);
    assert.ok(diagnostic.message.includes(`the path '${path}' cannot be used`), diagnostic.message);
    assert.match(diagnostic.message, problem, path);
  }
});

test("an arguments function with the wrong result type is rejected by the compiler", () => {
  const diagnostics = compileDiagnostics(`
/// Returns its argument.
let notArguments(orderId: string): string = { orderId }
let shop = <HttpConnection name="shop" baseUrl="https://api.example.com" />
let root(): Agent = { <Agent name="support" tools={ <HttpTool connection={shop} method={HttpMethod.get} path="/x" arguments={notArguments} /> }>Be brief.</Agent> }`);
  assert.equal(diagnostics.length, 1);
  assert.match(diagnostics[0]!.message, /expects <function \.\.\. \/>: HttpArguments/);
  assert.match(diagnostics[0]!.message, /the result string is not HttpArguments/);
});

test("normalization does not check the arguments function's result type, evaluate it or contact the connection", () => {
  // A value that did not come from a checked program: its arguments function returns a string.
  const compiled = agentProgram(
    `
/// Returns its argument.
let notArguments(orderId: string): string = { orderId }`,
    "<WebSearchTool />",
  );
  const unchecked = (agent: Agent): Agent =>
    handBuilt(agent, [
      {
        $type: "HttpTool",
        connection: { $type: "HttpConnection", name: "shop", baseUrl: "https://no-such-host.invalid" },
        method: "get",
        path: "/orders/{orderId}",
        arguments: { $type: "Function", module: "main.nx", name: "notArguments" },
      },
    ]);
  const tool = onlyTool(normalize(compiled, {}, unchecked));
  assert.equal(tool.name, "not_arguments");
  assert.equal(tool.kind, "http");
});

test("a malformed HTTP tool value is reported field by field, in one pass", () => {
  const compiled = httpProgram("https://api.example.com", getOrder);
  const malformed = (agent: Agent): Agent =>
    handBuilt(agent, [{ $type: "HttpTool", connection: "shop", method: "head", path: 7, arguments: "lookupOrder", maxCallsPerConversation: "three" }]);
  assert.deepEqual(
    refused(normalize(compiled, {}, malformed)).map((diagnostic) => [diagnostic.code, diagnostic.path]),
    [
      ["nx-agent-http-arguments-type", "tools[0].arguments"],
      ["nx-agent-http-base-url", "tools[0].connection"],
      ["nx-agent-unknown-tool-type", "tools[0].method"],
      ["nx-agent-http-path", "tools[0].path"],
      ["nx-agent-unknown-tool-type", "tools[0].maxCallsPerConversation"],
    ],
  );
});

// ---- host describers ----------------------------------------------------------------------------

const searchSchema = { type: "object", properties: { query: { type: "string" } }, required: ["query"], additionalProperties: false };
const readOnly = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false };

const recordSearch: NormalizeAgentOptions["toolTypes"] = {
  RecordSearchTool: (value, context) => {
    const { recordKind, maxResults } = value as unknown as { recordKind: string; maxResults: number };
    return {
      name: context.toSnakeCase(`search ${recordKind} records`),
      title: "Record search",
      description: `Searches ${recordKind} records.`,
      inputSchema: searchSchema,
      outputSchema: { type: "array" },
      annotations: readOnly,
      kind: "builtin",
      config: { builtin: "record_search", recordKind, maxResults },
    };
  },
};

test("a host tool type is described by its describer", () => {
  const seen: unknown[] = [];
  const compiled = agentProgram("", '<RecordSearchTool recordKind="company" />');
  const tool = onlyTool(
    normalize(compiled, {
      toolTypes: {
        RecordSearchTool: (value, context) => {
          seen.push([value, context.path, context.schemas === compiled.artifact, context.toSnakeCase("PlanRow")]);
          return recordSearch!["RecordSearchTool"]!(value, context);
        },
      },
    }),
  );
  assert.deepEqual(seen, [[{ $type: "RecordSearchTool", recordKind: "company", maxResults: 5 }, "tools[0]", true, "plan_row"]]);
  assert.deepEqual(tool, {
    name: "search_company_records",
    title: "Record search",
    description: "Searches company records.",
    inputSchema: searchSchema,
    outputSchema: { type: "array" },
    annotations: readOnly,
    kind: "builtin",
    config: { builtin: "record_search", recordKind: "company", maxResults: 5 },
  });
  assert.deepEqual(Object.keys(tool), ["name", "title", "description", "inputSchema", "outputSchema", "annotations", "kind", "config"]);
  // The definition holds copies, not the describer's own objects.
  assert.notEqual(tool.inputSchema, searchSchema);
});

test("the authored name and description override a single described tool's", () => {
  const compiled = agentProgram("", '<RecordSearchTool recordKind="company" name="find_companies" description="Find a company." />');
  const tool = onlyTool(normalize(compiled, { toolTypes: recordSearch }));
  assert.equal(tool.name, "find_companies");
  assert.equal(tool.description, "Find a company.");
  assert.equal(tool.title, "Record search");
});

const scheduler: NormalizeAgentOptions["toolTypes"] = {
  MeetingSchedulerTool: (value) =>
    ["find_meeting_times", "book_meeting"].map((name) => ({
      name,
      description: name === "book_meeting" ? "Books a meeting." : "Finds free times.",
      inputSchema: { type: "object", properties: {}, additionalProperties: false },
      annotations: { ...readOnly, readOnlyHint: name !== "book_meeting", destructiveHint: name === "book_meeting" },
      kind: "builtin",
      config: { builtin: name, calendar: (value as unknown as { calendar: string }).calendar },
    })),
};

test("one element resolves to two tools, in the order returned, at its position", () => {
  const compiled = agentProgram(plans, '<FunctionTool function={findPlans} /> <MeetingSchedulerTool calendar="sales" name="ignored" /> <WebSearchTool />');
  const definition = definitionOf(normalize(compiled, { toolTypes: scheduler }));
  assert.deepEqual(
    definition.tools.map((tool) => tool.name),
    ["find_plans", "find_meeting_times", "book_meeting", "web_search"],
  );
  assert.deepEqual(definition.tools[2]!.config, { builtin: "book_meeting", calendar: "sales" });
  assert.equal(definition.tools[2]!.annotations.destructiveHint, true);
});

test("an unregistered tool type is rejected, naming the type", () => {
  const compiled = agentProgram("", '<MeetingSchedulerTool calendar="sales" />');
  for (const options of [{}, { toolTypes: recordSearch }]) {
    const diagnostic = onlyDiagnostic(normalize(compiled, options));
    assert.equal(diagnostic.code, "nx-agent-unknown-tool-type");
    assert.equal(diagnostic.path, "tools[0]");
    assert.match(diagnostic.message, /'MeetingSchedulerTool'/);
  }
});

test("a describer registered under an inherited property name is not a describer", () => {
  const compiled = agentProgram("", "<WebSearchTool />");
  const named = (agent: Agent): Agent => handBuilt(agent, [{ $type: "toString" }, { $type: "constructor" }]);
  assert.deepEqual(
    refused(normalize(compiled, { toolTypes: {} }, named)).map((diagnostic) => diagnostic.code),
    ["nx-agent-unknown-tool-type", "nx-agent-unknown-tool-type"],
  );
});

test("a describer's diagnostics are reported against the tool", () => {
  const compiled = agentProgram("", '<RecordSearchTool recordKind="invoice" />');
  const result = normalize(compiled, {
    toolTypes: { RecordSearchTool: () => ({ diagnostics: [{ message: "'invoice' is not a record kind." }, { message: "Use 'company' or 'person'." }] }) },
  });
  assert.deepEqual(refused(result), [
    { severity: "error", code: "nx-agent-describer", message: "The describer of 'RecordSearchTool' refused tools[0]: 'invoice' is not a record kind.", path: "tools[0]" },
    { severity: "error", code: "nx-agent-describer", message: "The describer of 'RecordSearchTool' refused tools[0]: Use 'company' or 'person'.", path: "tools[0]" },
  ]);
});

test("a described tool is held to the package's rules for a name, a description and plain JSON", () => {
  const compiled = agentProgram("", '<RecordSearchTool recordKind="company" />');
  const base = { name: "search_records", description: "Searches.", inputSchema: searchSchema, annotations: readOnly, kind: "builtin", config: {} };
  const cases: readonly (readonly [object, string, RegExp])[] = [
    [{ ...base, name: "Search Records" }, "nx-agent-invalid-tool-name", /"Search Records"/],
    [{ ...base, description: "  " }, "nx-agent-missing-description", /no description/],
    [{ ...base, config: { when: new Date(0) } }, "nx-agent-describer", /the 'config' of 'search_records'\.when is not a plain object/],
    [{ ...base, config: { limit: Number.NaN } }, "nx-agent-describer", /\.limit is NaN, which JSON cannot hold/],
    [{ ...base, config: { run: () => 1 } }, "nx-agent-describer", /\.run is a function, which JSON cannot hold/],
    [{ ...base, config: { missing: undefined } }, "nx-agent-describer", /\.missing is undefined/],
    [{ ...base, inputSchema: undefined }, "nx-agent-describer", /has no 'inputSchema'/],
    [{ ...base, annotations: { readOnlyHint: true } }, "nx-agent-describer", /does not carry the four boolean annotations/],
    [{ ...base, kind: undefined }, "nx-agent-describer", /has no 'kind'/],
    [{ ...base, config: undefined }, "nx-agent-describer", /has no 'config'/],
    [{ ...base, name: undefined }, "nx-agent-describer", /has no 'name'/],
  ];
  for (const [description, code, message] of cases) {
    const diagnostic = onlyDiagnostic(normalize(compiled, { toolTypes: { RecordSearchTool: () => description as never } }));
    assert.equal(diagnostic.code, code, JSON.stringify(description));
    assert.equal(diagnostic.path, "tools[0]");
    assert.match(diagnostic.message, message);
  }
  const cyclic: Record<string, unknown> = {};
  cyclic["self"] = cyclic;
  assert.match(
    onlyDiagnostic(normalize(compiled, { toolTypes: { RecordSearchTool: () => ({ ...base, config: cyclic }) as never } })).message,
    /\.self holds itself/,
  );
  assert.match(onlyDiagnostic(normalize(compiled, { toolTypes: { RecordSearchTool: () => [] } })).message, /described no tool/);
  assert.match(onlyDiagnostic(normalize(compiled, { toolTypes: { RecordSearchTool: () => "search" as never } })).message, /not a tool description/);
});

test("a describer cannot take the package's kinds or replace its handling of its own types", () => {
  const compiled = agentProgram(plans, '<FunctionTool function={findPlans} /> <RecordSearchTool recordKind="company" />');
  const base = { name: "search_records", description: "Searches.", inputSchema: searchSchema, annotations: readOnly, config: {} };
  for (const kind of ["function", "http", "provider"]) {
    const diagnostic = onlyDiagnostic(normalize(compiled, { toolTypes: { RecordSearchTool: () => ({ ...base, kind }) } }));
    assert.equal(diagnostic.code, "nx-agent-describer");
    assert.match(diagnostic.message, new RegExp(`has the kind '${kind}', which is the package's own`));
  }
  for (const type of ["FunctionTool", "HttpTool", "WebSearchTool"]) {
    let called = false;
    const result = normalize(compiled, {
      toolTypes: {
        ...recordSearch,
        [type]: () => {
          called = true;
          return { ...base, kind: "builtin" };
        },
      },
    });
    const diagnostic = onlyDiagnostic(result);
    assert.equal(diagnostic.code, "nx-agent-describer");
    assert.equal(diagnostic.path, "");
    assert.match(diagnostic.message, new RegExp(`A describer is registered for '${type}', which the package describes itself`));
    assert.equal(called, false, type);
  }
});

test("what a describer throws is thrown on", () => {
  const compiled = agentProgram("", '<RecordSearchTool recordKind="company" />');
  const failure = new Error("the record index is down");
  assert.throws(
    () =>
      normalize(compiled, {
        toolTypes: {
          RecordSearchTool: () => {
            throw failure;
          },
        },
      }),
    (error) => error === failure,
  );
});

// ---- names across the agent, and the definition as stored ----------------------------------------

test("duplicate names are rejected with one diagnostic naming the name and both positions", () => {
  const compiled = agentProgram(plans, '<FunctionTool function={findPlans} /> <WebSearchTool /> <FunctionTool function={findPlans} description="Again." />');
  assert.deepEqual(refused(normalize(compiled)), [
    {
      severity: "error",
      code: "nx-agent-duplicate-tool-name",
      message: "The tool name 'find_plans' is used by tools[0] and tools[2]; each tool of an agent needs its own name.",
      path: "tools[2]",
      tool: "find_plans",
    },
  ]);
});

test("a name is unique across derived, authored and described tools", () => {
  const derivedAndAuthored = agentProgram(plans, '<FunctionTool function={findPlans} /> <WebSearchTool name="find_plans" />');
  assert.equal(onlyDiagnostic(normalize(derivedAndAuthored)).code, "nx-agent-duplicate-tool-name");

  const describedAsFindPlans: NormalizeAgentOptions["toolTypes"] = {
    RecordSearchTool: (value, context) => ({ ...(recordSearch!["RecordSearchTool"]!(value, context) as object), name: "find_plans" }) as never,
  };
  const derivedAndDescribed = agentProgram(plans, '<RecordSearchTool recordKind="company" /> <FunctionTool function={findPlans} />');
  const diagnostic = onlyDiagnostic(normalize(derivedAndDescribed, { toolTypes: describedAsFindPlans }));
  assert.equal(diagnostic.code, "nx-agent-duplicate-tool-name");
  assert.match(diagnostic.message, /'find_plans' is used by tools\[0\] and tools\[1\]/);

  const twoOfOneElement = agentProgram("", '<MeetingSchedulerTool calendar="sales" /> <WebSearchTool name="book_meeting" />');
  assert.match(onlyDiagnostic(normalize(twoOfOneElement, { toolTypes: scheduler })).message, /'book_meeting' is used by tools\[0\] and tools\[1\]/);

  const three = agentProgram(plans, '<FunctionTool function={findPlans} /> <WebSearchTool name="find_plans" /> <FunctionTool function={findPlans} name="find_plans" />');
  assert.match(onlyDiagnostic(normalize(three)).message, /used by tools\[0\], tools\[1\] and tools\[2\]/);
});

test("two problems are reported together, each with its own code and path", () => {
  const compiled = agentProgram(
    `${plans}\nlet undocumented(n:int): int = { n }`,
    "<FunctionTool function={undocumented} /> <FunctionTool function={findPlans} /> <FunctionTool function={findPlans} />",
  );
  assert.deepEqual(
    refused(normalize(compiled)).map((diagnostic) => [diagnostic.code, diagnostic.path, diagnostic.tool]),
    [
      ["nx-agent-missing-description", "tools[0]", "undocumented"],
      ["nx-agent-duplicate-tool-name", "tools[2]", "find_plans"],
    ],
  );
});

test("every problem of one agent is reported in one pass", () => {
  const compiled = agentProgram(
    `${plans}${contextFunctions}
let undocumented(n:int): int = { n }
/// Applies a function.
let applyTo(f: <function n:int />: int, n:int): int = <f n={n} />
/// Triples a number.
let triple(n:int): int = { n * 3 }
/// Picks a function.
let pick(n:int): <function n:int />: int = { triple }
let shop = <HttpConnection name="shop" baseUrl="http://api.example.com?key=1" />
/// Looks an order up.
let lookupOrder(orderId: string): HttpArguments = { <HttpArguments /> }`,
    `<FunctionTool function={undocumented} />
     <FunctionTool function={findPlans} name="Find Plans" />
     <FunctionTool function={applyTo} />
     <FunctionTool function={orderFor} />
     <FunctionTool function={pick} />
     <HttpTool connection={shop} method={HttpMethod.get} path="/orders/../{id" arguments={lookupOrder} />
     <MeetingSchedulerTool calendar="sales" />
     <WebSearchTool />
     <WebSearchTool />`,
  );
  const result = normalize(compiled);
  assert.equal(result.ok, false);
  assert.equal("definition" in result, false);
  assert.deepEqual(
    result.diagnostics.map((diagnostic) => [diagnostic.severity, diagnostic.code, diagnostic.path]),
    [
      ["error", "nx-agent-missing-description", "tools[0]"],
      ["error", "nx-agent-invalid-tool-name", "tools[1].name"],
      ["error", "nx-agent-inexpressible-parameter", "tools[2]"],
      ["error", "nx-agent-context-parameter", "tools[3]"],
      ["warning", "nx-agent-inexpressible-result", "tools[4]"],
      ["error", "nx-agent-http-base-url", "tools[5].connection.baseUrl"],
      ["error", "nx-agent-http-path", "tools[5].path"],
      ["error", "nx-agent-unknown-tool-type", "tools[6]"],
      ["error", "nx-agent-duplicate-tool-name", "tools[8]"],
    ],
  );
  for (const diagnostic of result.diagnostics) {
    assert.match(diagnostic.code, /^nx-agent-/);
    assert.equal(typeof diagnostic.message, "string");
  }
});

test("a warning does not block the definition", () => {
  const compiled = agentProgram(
    `${plans}
/// Triples a number.
let triple(n:int): int = { n * 3 }
/// Picks a function.
let pick(n:int): <function n:int />: int = { triple }`,
    "<FunctionTool function={findPlans} /> <FunctionTool function={pick} />",
  );
  const result = normalize(compiled);
  assert.equal(definitionOf(result).tools.length, 2);
  assert.deepEqual(result.diagnostics.map((diagnostic) => [diagnostic.severity, diagnostic.code]), [["warning", "nx-agent-inexpressible-result"]]);
});

const everyKind = `${plans}${orders}${contextFunctions}
let shop = <HttpConnection name="shop" baseUrl="https://api.example.com/v1/" />
let refundPolicy = <Document title="Refund policy">Refunds are available within 30 days of purchase.</Document>
let root(): Agent = {
  <Agent name="support" description="Answers customer questions." model="balanced"
      documents={ refundPolicy }
      tools={
        <FunctionTool function={findPlans} />
        <FunctionTool function={orderFor} />
        <WebSearchTool allowedDomains={ "docs.example.com" } />
        <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupMine} maxCallsPerConversation=3 />
        <RecordSearchTool recordKind="company" />
        <MeetingSchedulerTool calendar="sales" />
      }
      limits={ <AgentLimits maxSteps=6 /> }>
    Be brief.
  </Agent>
}`;
const everyDescriber = { ...recordSearch, ...scheduler };

test("the definition survives storage: it is plain JSON, and parsing it back gives an equal definition", () => {
  const compiled = compileProgram(everyKind);
  const definition = definitionOf(normalizeAgent(compiled.agent(), { schemas: compiled.artifact, toolContextType: chatToolContext, toolTypes: everyDescriber }));
  assert.equal(definition.tools.length, 7);
  const text = JSON.stringify(definition);
  assert.deepEqual(JSON.parse(text), definition);
  assert.equal(JSON.stringify(JSON.parse(text)), text);
  assert.equal(text.includes("null"), false);
  assert.equal(text.includes('"title"'), true, "the described tool keeps its title");
  assert.deepEqual(definition.limits, { maxSteps: 6 });
});

test("normalization is deterministic: two runs give the same JSON text", () => {
  const options = { toolContextType: chatToolContext, toolTypes: everyDescriber };
  const first = compileProgram(everyKind);
  const second = compileProgram(everyKind);
  const texts = [
    JSON.stringify(definitionOf(normalizeAgent(first.agent(), { schemas: first.artifact, ...options }))),
    JSON.stringify(definitionOf(normalizeAgent(first.agent(), { schemas: first.artifact, ...options }))),
    JSON.stringify(definitionOf(normalizeAgent(second.agent(), { schemas: second.artifact, ...options }))),
  ];
  assert.equal(texts[1], texts[0]);
  assert.equal(texts[2], texts[0]);
});

test("the definition is the package's alone: changing the value afterwards does not change it", () => {
  const compiled = agentProgram("", '<WebSearchTool allowedDomains={ "docs.example.com" } />');
  const agent = JSON.parse(JSON.stringify(compiled.agent())) as Agent;
  const definition = definitionOf(normalizeAgent(agent, { schemas: compiled.artifact }));
  const before = JSON.stringify(definition);
  (agent.tools![0] as unknown as { allowedDomains: string[] }).allowedDomains.push("evil.example");
  assert.equal(JSON.stringify(definition), before);
});
