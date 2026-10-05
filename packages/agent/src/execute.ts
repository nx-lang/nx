/**
 * The run-time half of `@nx-lang/agent`: turns a stored definition and a linked IR program into
 * tools a host can run. It needs `@nx-lang/ir-runtime` and nothing else: no compiler, no schemas
 * and no NX source.
 *
 * UNSTABLE. The API and the stored definition format may change in any release.
 */
import {
  NxIrRuntimeError,
  callFunction,
  measureInputSize,
  type NxPreparedProgram,
  type NxRuntimeOptions,
  type NxRuntimeUsage,
} from "@nx-lang/ir-runtime";

import { classifyRuntimeFailure } from "./classify.js";
import {
  AGENT_DEFINITION_FORMAT_VERSION,
  PACKAGE_TOOL_KINDS,
  isFunctionTool,
  isHttpTool,
  isProviderTool,
  type ContextParameter,
  type NormalizedAgent,
  type NormalizedHttpTool,
  type NormalizedTool,
  type NxAgentDeclarationName,
  type ToolAnnotations,
} from "./definition.js";
import { NxAgentError, NxAgentToolError, type NxAgentDiagnostic, type ToolError, type ToolUsage } from "./diagnostics.js";
import { buildHttpCall, type HttpArgumentsData, type HttpRequestDescription } from "./http.js";
import { describeValue, isJsonObject, withoutUndefinedMembers, type JsonValue } from "./json.js";

/**
 * The operations one tool call may cost when the host's runtime options set no `maxOperations`.
 * The IR runtime has no default of its own, and this package exists to run code its host did not
 * write. A realistic tool costs hundreds to a few thousand operations, so this is ample, and low
 * on purpose: a tool that outgrows it fails with a `limit` that says so.
 */
export const NX_AGENT_DEFAULT_MAX_OPERATIONS = 100_000;

/** The largest input a model may send to one tool call, as `measureInputSize` measures it, by default. */
export const NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE = 1_000;

/** The largest tool context record a host may pass to one tool call, with `callId` set, by default. */
export const NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE = 1_000;

/**
 * The host's tool context: a record of the concrete `ToolContext` subtype it declares, with its
 * `$type`. Nothing else of it is typed here, since its fields are the host's.
 *
 * <para>A value of the interface `nxlang typegen` writes for that subtype is one, with its
 * `callId` or without it (`Omit<ChatToolContext, "callId">`), since the package sets `callId` at
 * every call. So is an object literal: the second member of the union is what lets one name its
 * fields, which an interface, having no index signature, could not be assigned to.</para>
 */
export type ToolContextRecord = { readonly $type: string } | { readonly $type: string; readonly [field: string]: JsonValue };

/** What a host passes to one call of a tool. */
export interface ToolCallContext {
  /**
   * Identifies the call. The host chooses it, and keeps it the same when it retries the call. The
   * package never makes one up, changes it or remembers it.
   */
  readonly callId: string;
  /**
   * The host's tool context record, with its `$type`. It is passed, with `callId` set to this
   * call's, to every context parameter of the tool's function. A tool with none ignores it. A
   * member that is `undefined` is left out, as an optional field with no value is: of the record
   * itself, whatever built it, and of every plain object below it. An instance of a class below
   * the top level is passed as it is, so a nested part of the context is written as a plain
   * object.
   */
  readonly context?: ToolContextRecord | undefined;
  /** Checked before anything is evaluated, and handed to a request function or a host executor. */
  readonly signal?: AbortSignal | undefined;
}

/**
 * How a tool call ended. A failure of the tool is a result, never a rejected promise. `usage` is
 * present whenever the call reached the IR runtime, on a success and on a failure alike, and absent
 * when the package refused the call before that, and for a tool a host executor runs.
 */
export type ToolResult =
  | { readonly ok: true; readonly output: JsonValue; readonly usage?: ToolUsage }
  | { readonly ok: false; readonly error: ToolError; readonly usage?: ToolUsage };

/** A tool of a stored definition, with `execute` unless the model's provider runs it. */
export type AgentTool = NormalizedTool & {
  readonly execute?: (input: unknown, context: ToolCallContext) => Promise<ToolResult>;
};

/** What an `http` tool's request function is handed for one call. */
export interface AgentHttpRequestCall {
  /** The request the package built: method, URL, headers and, when there is one, the body as JSON text. */
  readonly request: HttpRequestDescription;
  /** The arguments as the tool's function returned them, in structured form. `body` keeps its `$type` keys. */
  readonly arguments: HttpArgumentsData;
  readonly callId: string;
  readonly toolName: string;
  /** The name of the tool's connection. */
  readonly connection: string;
  readonly annotations: ToolAnnotations;
  readonly signal?: AbortSignal;
}

/** What a request function answers: the response's status and body, and anything the host adds. */
export type AgentHttpResponse = { readonly status: number; readonly body: JsonValue; readonly [key: string]: JsonValue };

/**
 * Makes the request of an `http` tool. The package builds the request and calls this; it never
 * calls `fetch`, and applies no network policy: addresses, redirects, timeouts, sizes, credentials,
 * idempotency headers, retries and call limits are the host's, here.
 *
 * <para>What it answers is the tool's output, whatever the status. It throws an
 * {@link NxAgentToolError} to fail the call with a code of its own; anything else it throws fails
 * the call as `request-failed`.</para>
 */
export type AgentHttpRequestFunction = (call: AgentHttpRequestCall) => Promise<AgentHttpResponse>;

/**
 * Runs a tool of a host's own kind. It is given the tool's definition, with the `config` its
 * describer stored, the model's input as it came, and the call context. What it returns is the
 * tool's output; what it throws fails the call, an {@link NxAgentToolError} with its own code.
 */
export type AgentToolExecutor = (tool: NormalizedTool, input: unknown, context: ToolCallContext) => JsonValue | Promise<JsonValue>;

/** The limits on what goes into one call, and the IR runtime's options for it. A member that is `undefined` is one that is absent. */
export interface AgentEvaluationOptions {
  /**
   * The IR runtime's options for every call. `maxOperations` is the budget of one call, and
   * {@link NX_AGENT_DEFAULT_MAX_OPERATIONS} when it is not set. `maxInputSize` and `usage` are not
   * used: the package sets the first from the two limits below and gives each call a `usage` of
   * its own, which it puts on the result.
   */
  readonly runtime?: NxRuntimeOptions | undefined;
  /** The largest input the model may send. {@link NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE} by default. */
  readonly maxArgumentsSize?: number | undefined;
  /** The largest tool context record. {@link NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE} by default. */
  readonly maxContextSize?: number | undefined;
}

export interface CreateAgentToolsOptions extends AgentEvaluationOptions {
  /** Makes the requests of `http` tools. Required when the definition holds one. */
  readonly request?: AgentHttpRequestFunction | undefined;
  /** Executors for the host's own kinds, by `kind`. Not `function`, `http` or `provider`. */
  readonly executors?: Readonly<Record<string, AgentToolExecutor>> | undefined;
}

/** What `evaluateHttpArguments` answers: the structured arguments and the request, or the failure `execute` would give. */
export type EvaluateHttpArgumentsResult =
  | { readonly ok: true; readonly arguments: HttpArgumentsData; readonly request: HttpRequestDescription; readonly usage?: ToolUsage }
  | { readonly ok: false; readonly error: ToolError; readonly usage?: ToolUsage };

const SUPPORTED_FORMAT_VERSIONS: readonly number[] = [AGENT_DEFINITION_FORMAT_VERSION];

/** A failed result. A member of `details` that is `undefined` is left out, as every absent field of a result is. */
function failure(
  code: ToolError["code"],
  message: string,
  details: { readonly diagnostics?: ToolError["diagnostics"] | undefined; readonly limit?: ToolError["limit"] | undefined } = {},
  usage?: ToolUsage,
): ToolResult & { ok: false } {
  return {
    ok: false,
    error: {
      code,
      message,
      ...(details.diagnostics === undefined ? {} : { diagnostics: details.diagnostics }),
      ...(details.limit === undefined ? {} : { limit: details.limit }),
    },
    ...(usage === undefined ? {} : { usage }),
  };
}

/** The failure a thrown value is: the code of an `NxAgentToolError`, or `fallback` for anything else. */
function thrownFailure(error: unknown, fallback: ToolError["code"], usage?: ToolUsage): ToolResult & { ok: false } {
  if (error instanceof NxAgentToolError) {
    return failure(error.code, error.message, { diagnostics: error.diagnostics, limit: error.limit }, usage);
  }
  return failure(fallback, error instanceof Error ? error.message : String(error), {}, usage);
}

/** A limit a host set, or its default: a non-negative safe integer, so it cannot be turned off. */
function limitOption(name: string, value: number | undefined, fallback: number): number {
  if (value === undefined) {
    return fallback;
  }
  if (!(Number.isSafeInteger(value) && value >= 0)) {
    throw new RangeError(`${name} must be a non-negative safe integer, got ${String(value)}.`);
  }
  return value;
}

/** The limits of every call made under one set of options, read and checked once. */
interface Limits {
  readonly runtime: NxRuntimeOptions;
  readonly maxOperations: number;
  readonly maxArgumentsSize: number;
  readonly maxContextSize: number;
}

function readLimits(options: AgentEvaluationOptions): Limits {
  const runtime = options.runtime ?? {};
  return {
    runtime,
    maxOperations: limitOption("runtime.maxOperations", runtime.maxOperations, NX_AGENT_DEFAULT_MAX_OPERATIONS),
    maxArgumentsSize: limitOption("maxArgumentsSize", options.maxArgumentsSize, NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE),
    maxContextSize: limitOption("maxContextSize", options.maxContextSize, NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE),
  };
}

/** What a field named `name` adds to the size of the record that holds it, beyond its value. */
function nameCost(name: string): number {
  // Measured, not computed, so the rule stays the runtime's: a record of one field holding one
  // number is the record, the number and whatever the name costs.
  return measureInputSize({ [name]: 0 }) - 2;
}

/** The outcome of evaluating a tool's function: its result and what the call used, or the failure. */
type Evaluated = { readonly ok: true; readonly value: JsonValue; readonly usage: ToolUsage } | (ToolResult & { ok: false });

/**
 * Prepares the calls of one function: the function a `function` tool names, or the arguments
 * function of an `http` tool. Everything that does not depend on a call is worked out here, once.
 */
function prepareFunctionCalls(
  toolName: string,
  reference: NxAgentDeclarationName,
  contextParameters: readonly ContextParameter[],
  program: NxPreparedProgram,
  limits: Limits,
): (input: unknown, context: ToolCallContext) => Evaluated {
  const record = { $type: "Function", module: reference.module, name: reference.name } as const;
  const contextNames = new Set(contextParameters.map((parameter) => parameter.name));
  // The most a call within both limits can be: the model's input at its cap; the context at its
  // allowance, once for each parameter it is passed to, with what that parameter's name costs as a
  // field of the arguments; and the function record. The runtime's own limit is set to this, so it
  // is a backstop a call that passed the two checks below cannot reach.
  const maxInputSize =
    limits.maxArgumentsSize +
    contextParameters.reduce((size, parameter) => size + limits.maxContextSize + nameCost(parameter.name), 0) +
    measureInputSize(record);

  return (input, context) => {
    if (!isJsonObject(input)) {
      return failure("invalid-input", `Tool '${toolName}' takes a JSON object as its input.`);
    }
    if (measureInputSize(input, limits.maxArgumentsSize) > limits.maxArgumentsSize) {
      return failure("invalid-input", `The input of tool '${toolName}' is larger than its limit of ${limits.maxArgumentsSize}.`, {
        limit: { name: "maxArgumentsSize", value: limits.maxArgumentsSize },
      });
    }

    // A key of the model's input that names a context parameter is dropped: only the host fills one.
    const entries = Object.entries(input).filter(([key]) => !contextNames.has(key));
    if (contextParameters.length > 0) {
      const supplied: unknown = context.context;
      if (!isJsonObject(supplied) || typeof supplied["$type"] !== "string") {
        const names = contextParameters.map((parameter) => `'${parameter.name}'`).join(", ");
        return failure(
          "invalid-context",
          `Tool '${toolName}' needs the host's tool context record, with its '$type', for its parameter ${names}; the call supplied ${supplied === undefined ? "none" : "one with no '$type'"}.`,
        );
      }
      // A record of more values than this is over the limit however it is measured: a value
      // costs at least one, but for a `$type`, of which a record has one. Copying no further is
      // what ends on a record that holds itself.
      // The context record itself is read as a record whatever built it, an instance of a
      // host's class included: its own members are what is passed. Below it, only what the
      // runtime's measure enters is looked into.
      const copied = withoutUndefinedMembers(supplied, 2 * limits.maxContextSize + 2, true);
      const contextRecord = copied === undefined ? undefined : { ...(copied as typeof supplied), callId: context.callId };
      if (contextRecord === undefined || measureInputSize(contextRecord, limits.maxContextSize) > limits.maxContextSize) {
        return failure("resource-limit", `The tool context passed to tool '${toolName}' is larger than its limit of ${limits.maxContextSize}.`, {
          limit: { name: "maxContextSize", value: limits.maxContextSize },
        });
      }
      for (const parameter of contextParameters) {
        entries.push([parameter.name, contextRecord]);
      }
    }

    // Each call reports to an object of its own. One shared by calls a model makes together would
    // hold the numbers of whichever ended last, and the host's own is not written to.
    const sink: NxRuntimeUsage = {};
    const usage = (): ToolUsage => ({
      ...(sink.operations === undefined ? {} : { operations: sink.operations }),
      ...(sink.inputSize === undefined ? {} : { inputSize: sink.inputSize }),
    });
    try {
      // `fromEntries` defines each key as a property of its own, `__proto__` included.
      const value = callFunction(program, record, Object.fromEntries(entries), {
        ...limits.runtime,
        maxOperations: limits.maxOperations,
        maxInputSize,
        usage: sink,
      });
      return { ok: true, value, usage: usage() };
    } catch (error) {
      if (!(error instanceof NxIrRuntimeError)) {
        return thrownFailure(error, "evaluation-failed", usage());
      }
      const { diagnostics } = error;
      const { code, limit } = classifyRuntimeFailure(diagnostics);
      return failure(code, error.message, { diagnostics, limit }, usage());
    }
  };
}

/** What stops a call before anything else is looked at: an abort, or no `callId`. */
function refusedCall(toolName: string, context: ToolCallContext | undefined): (ToolResult & { ok: false }) | undefined {
  if (context?.signal?.aborted === true) {
    return failure("aborted", `The call of tool '${toolName}' was aborted before it began.`);
  }
  if (typeof context?.callId !== "string" || context.callId === "") {
    return failure("invalid-context", `A call of tool '${toolName}' needs a 'callId', a string that is not empty.`);
  }
  return undefined;
}

/** Evaluates an `http` tool's arguments function and builds its request. */
function prepareHttpCalls(
  tool: NormalizedHttpTool,
  program: NxPreparedProgram,
  limits: Limits,
): (input: unknown, context: ToolCallContext) => EvaluateHttpArgumentsResult {
  const { arguments: reference, contextParameters } = tool.config;
  const evaluate = prepareFunctionCalls(tool.name, reference, contextParameters, program, limits);
  return (input, context) => {
    const evaluated = evaluate(input, context);
    if (!evaluated.ok) {
      return evaluated;
    }
    const { value, usage } = evaluated;
    // The compiler checks this where a program binds the function. A value that did not come from
    // a checked program, or a definition run against a program that has changed, is caught here.
    if (!isJsonObject(value) || value["$type"] !== "HttpArguments") {
      return failure(
        "invalid-request",
        `Tool '${tool.name}': its arguments function '${reference.name}' of module '${reference.module}' returned ${describeValue(value)}, not an 'HttpArguments'.`,
        {},
        usage,
      );
    }
    const built = buildHttpCall(tool, value);
    if (!built.ok) {
      return failure("invalid-request", built.error.message, {}, usage);
    }
    return { ok: true, arguments: built.arguments, request: built.request, usage };
  };
}

/** Whether `value` names a declaration the way a definition stores one: `{ module, name }`. */
function isDeclarationName(value: unknown): value is NxAgentDeclarationName {
  return isJsonObject(value) && typeof value["module"] === "string" && typeof value["name"] === "string";
}

/**
 * What is wrong with the shape of one stored tool, or nothing. A definition is JSON a host kept
 * and parsed, so its type is a claim. This checks every member the package reads before a call is
 * made; what a call reads, the request builder checks at the call.
 */
function toolShapeProblem(tool: unknown): string | undefined {
  if (!isJsonObject(tool)) {
    return `it is ${describeValue(tool)}, not a tool`;
  }
  for (const member of ["name", "kind"]) {
    if (typeof tool[member] !== "string") {
      return `its '${member}' is ${describeValue(tool[member])}, not a string`;
    }
  }
  if (tool["kind"] !== "function" && tool["kind"] !== "http") {
    // A provider tool is described only, and a host's own tool is its executor's to read.
    return undefined;
  }
  const config = tool["config"];
  if (!isJsonObject(config)) {
    return `its 'config' is ${describeValue(config)}, not a record`;
  }
  const reference = tool["kind"] === "function" ? "function" : "arguments";
  if (!isDeclarationName(config[reference])) {
    return `its 'config.${reference}' is ${describeValue(config[reference])}, not the module and the name of a function`;
  }
  const contextParameters = config["contextParameters"];
  if (!Array.isArray(contextParameters) || !contextParameters.every((parameter) => isJsonObject(parameter) && typeof parameter["name"] === "string")) {
    return `its 'config.contextParameters' is ${Array.isArray(contextParameters) ? "a list that holds something other than a parameter with a 'name'" : `${describeValue(contextParameters)}, not a list`}`;
  }
  const connection = config["connection"];
  if (tool["kind"] === "http" && !(isJsonObject(connection) && typeof connection["name"] === "string")) {
    return `its 'config.connection' is ${describeValue(connection)}, not a connection with a 'name'`;
  }
  return undefined;
}

/**
 * The problems of a definition that do not depend on how it will be run: a format version the
 * package does not support, a definition or a tool that does not have the shape the package wrote,
 * and a `function` or `http` tool whose function the program does not declare.
 */
function definitionProblems(
  definition: NormalizedAgent | readonly NormalizedTool[],
  program: NxPreparedProgram,
): {
  readonly tools: readonly NormalizedTool[];
  /** The places in `tools` of those that do not have a tool's shape, and so cannot be read further. */
  readonly damaged: ReadonlySet<number>;
  readonly diagnostics: NxAgentDiagnostic[];
} {
  const diagnostics: NxAgentDiagnostic[] = [];
  const damaged = new Set<number>();
  const malformed = (path: string, message: string, tool?: string): void => {
    diagnostics.push({ severity: "error", code: "nx-agent-invalid-definition", message, path, ...(tool === undefined ? {} : { tool }) });
  };
  let tools: readonly NormalizedTool[];
  if (Array.isArray(definition)) {
    tools = definition as readonly NormalizedTool[];
  } else if (!isJsonObject(definition)) {
    malformed("", `The definition is ${describeValue(definition)}, not a definition or a list of tools.`);
    return { tools: [], damaged, diagnostics };
  } else {
    const { formatVersion } = definition as { formatVersion?: unknown };
    if (!SUPPORTED_FORMAT_VERSIONS.includes(formatVersion as number)) {
      // Nothing else in a definition of another format can be read with any confidence.
      diagnostics.push({
        severity: "error",
        code: "nx-agent-unsupported-format",
        message: `The definition has format version ${JSON.stringify(formatVersion) ?? "undefined"}, and this package supports ${SUPPORTED_FORMAT_VERSIONS.join(", ")}.`,
        path: "formatVersion",
      });
      return { tools: [], damaged, diagnostics };
    }
    const held = (definition as { tools?: unknown }).tools;
    if (!Array.isArray(held)) {
      malformed("tools", `The definition's 'tools' is ${describeValue(held)}, not a list.`);
      return { tools: [], damaged, diagnostics };
    }
    tools = held as readonly NormalizedTool[];
  }

  for (const [index, tool] of tools.entries()) {
    const problem = toolShapeProblem(tool);
    if (problem !== undefined) {
      const name: unknown = isJsonObject(tool) ? tool["name"] : undefined;
      malformed(`tools[${index}]`, `tools[${index}] of the definition cannot be run: ${problem}.`, typeof name === "string" ? name : undefined);
      damaged.add(index);
      continue;
    }
    const reference = isFunctionTool(tool) ? tool.config.function : isHttpTool(tool) ? tool.config.arguments : undefined;
    if (reference === undefined) {
      continue;
    }
    // The one read of the runtime's prepared types: two maps and a declaration's kind.
    const declaration = program.modulesByIdentity.get(reference.module)?.module.declarationsByName.get(reference.name);
    if (declaration?.kind.tag !== "function") {
      diagnostics.push({
        severity: "error",
        code: "nx-agent-unknown-function",
        message: `Tool '${tool.name}' names the function '${reference.name}' of module '${reference.module}', which the linked program does not declare.`,
        path: `tools[${index}]`,
        tool: tool.name,
        declaration: { module: reference.module },
      });
    }
  }
  return { tools, damaged, diagnostics };
}

/**
 * Checks a definition against the program it will run with, without anything needed to run it: no
 * executors and no request function. A host calls it after it links what it compiled.
 *
 * <para>It answers the diagnostics for a format version the package does not support, for a
 * definition or a tool that does not have the shape the package wrote, and for each `function` and
 * `http` tool whose function the linked program does not declare, and none when the definition can
 * be given to `createAgentTools` with this program.</para>
 */
export function checkAgentTools(definition: NormalizedAgent | readonly NormalizedTool[], program: NxPreparedProgram): NxAgentDiagnostic[] {
  return definitionProblems(definition, program).diagnostics;
}

/**
 * Makes the tools of a stored definition executable over a linked program.
 *
 * <para>It answers one tool for each of the definition's, in order, with the definition's fields
 * and, for every kind but `provider`, an `execute(input, context)`. A `function` tool calls its
 * function through the IR runtime; an `http` tool evaluates its arguments function, builds the
 * request and hands it to `options.request`; a tool of a host's kind is run by the executor
 * registered for it; a `provider` tool is described only.</para>
 *
 * <para>`execute` resolves to a result and does not reject for a failure of the tool. A call needs
 * a `callId` and is refused when its signal is already aborted.</para>
 *
 * @throws NxAgentError before any tool runs, with every problem found: `nx-agent-unsupported-format`,
 * `nx-agent-invalid-definition`, `nx-agent-unknown-function`, `nx-agent-missing-executor`,
 * `nx-agent-missing-request-function`.
 * @throws TypeError when `options.executors` holds `function`, `http` or `provider`, which the
 * package runs or describes itself.
 * @throws RangeError when a limit is not a non-negative safe integer.
 */
export function createAgentTools(
  definition: NormalizedAgent | readonly NormalizedTool[],
  program: NxPreparedProgram,
  options: CreateAgentToolsOptions = {},
): AgentTool[] {
  const limits = readLimits(options);
  const executors = options.executors ?? {};
  for (const kind of PACKAGE_TOOL_KINDS) {
    if (Object.hasOwn(executors, kind)) {
      throw new TypeError(`An executor is registered for the kind '${kind}', which the package ${kind === "provider" ? "describes and never runs" : "runs itself"}.`);
    }
  }

  const { tools, damaged, diagnostics } = definitionProblems(definition, program);
  for (const [index, tool] of tools.entries()) {
    if (damaged.has(index)) {
      // Already reported, and its kind cannot be trusted to say what it needs.
      continue;
    }
    const path = `tools[${index}]`;
    if (isHttpTool(tool) && options.request === undefined) {
      diagnostics.push({
        severity: "error",
        code: "nx-agent-missing-request-function",
        message: `Tool '${tool.name}' is an http tool, and no 'request' function was supplied to make its requests.`,
        path,
        tool: tool.name,
      });
    } else if (!PACKAGE_TOOL_KINDS.includes(tool.kind) && !Object.hasOwn(executors, tool.kind)) {
      diagnostics.push({
        severity: "error",
        code: "nx-agent-missing-executor",
        message: `Tool '${tool.name}' is of kind '${String(tool.kind)}', and no executor is registered for that kind.`,
        path,
        tool: tool.name,
      });
    }
  }
  if (diagnostics.length > 0) {
    // No tools: a tool that cannot run is never left out, and never left to fail at its first call.
    throw new NxAgentError(diagnostics);
  }

  return tools.map((tool): AgentTool => {
    if (isProviderTool(tool)) {
      return { ...tool };
    }
    if (isFunctionTool(tool)) {
      const evaluate = prepareFunctionCalls(tool.name, tool.config.function, tool.config.contextParameters, program, limits);
      return {
        ...tool,
        execute: async (input, context) => {
          const refused = refusedCall(tool.name, context);
          if (refused !== undefined) {
            return refused;
          }
          const evaluated = evaluate(input, context);
          return evaluated.ok ? { ok: true, output: evaluated.value, usage: evaluated.usage } : evaluated;
        },
      };
    }
    if (isHttpTool(tool)) {
      const evaluate = prepareHttpCalls(tool, program, limits);
      const request = options.request!;
      return {
        ...tool,
        execute: async (input, context) => {
          const refused = refusedCall(tool.name, context);
          if (refused !== undefined) {
            return refused;
          }
          const evaluated = evaluate(input, context);
          if (!evaluated.ok) {
            return evaluated;
          }
          const { usage } = evaluated;
          let response: unknown;
          try {
            response = await request({
              request: evaluated.request,
              arguments: evaluated.arguments,
              callId: context.callId,
              toolName: tool.name,
              connection: tool.config.connection.name,
              annotations: tool.annotations,
              ...(context.signal === undefined ? {} : { signal: context.signal }),
            });
          } catch (error) {
            return thrownFailure(error, "request-failed", usage);
          }
          if (!isJsonObject(response) || typeof response["status"] !== "number") {
            return failure("request-failed", `The request function answered tool '${tool.name}' with something that has no numeric 'status'.`, {}, usage);
          }
          // The whole answer is the output, whatever its status and whatever else the host put on it.
          return { ok: true, output: response, ...(usage === undefined ? {} : { usage }) };
        },
      };
    }
    const executor = executors[tool.kind]!;
    return {
      ...tool,
      execute: async (input, context) => {
        const refused = refusedCall(tool.name, context);
        if (refused !== undefined) {
          return refused;
        }
        try {
          const output = await executor(tool, input, context);
          return { ok: true, output: output === undefined ? null : output };
        } catch (error) {
          return thrownFailure(error, "evaluation-failed");
        }
      },
    };
  });
}

/**
 * The first two steps of an `http` tool's `execute`, on their own: evaluates the tool's arguments
 * function and builds the request, and sends nothing. It takes no request function.
 *
 * <para>It answers the structured arguments and the request, or the failure `execute` would give
 * for the same call: under the same budget and the same two input limits, with the same codes,
 * and with the `usage` of the arguments function's call whenever that call was made.</para>
 *
 * @throws RangeError when a limit is not a non-negative safe integer.
 */
export function evaluateHttpArguments(
  tool: NormalizedTool,
  program: NxPreparedProgram,
  input: unknown,
  context: ToolCallContext,
  options: AgentEvaluationOptions = {},
): EvaluateHttpArgumentsResult {
  const limits = readLimits(options);
  const problem = toolShapeProblem(tool);
  if (problem !== undefined) {
    return failure("invalid-request", `The tool cannot be run: ${problem}.`);
  }
  if (!isHttpTool(tool)) {
    return failure("invalid-request", `Tool '${tool.name}' is of kind '${String(tool.kind)}', not 'http'.`);
  }
  const refused = refusedCall(tool.name, context);
  if (refused !== undefined) {
    return refused;
  }
  return prepareHttpCalls(tool, program, limits)(input, context);
}
