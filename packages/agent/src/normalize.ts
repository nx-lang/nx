/**
 * The compile-time half of `@nx-lang/agent`: turns an evaluated `Agent` value into a definition a
 * host stores. It loads no compiler; the host supplies declaration schemas through an option,
 * which in practice is the program artifact it built.
 *
 * UNSTABLE. The API and the stored definition format may change in any release.
 */
import {
  FUNCTION_TOOL_ANNOTATIONS,
  WEB_SEARCH_TOOL_ANNOTATIONS,
  httpToolAnnotations,
  isHttpToolMethod,
} from "./annotations.js";
import {
  AGENT_DEFINITION_FORMAT_VERSION,
  NX_AGENT_TOOL_CONTEXT,
  PACKAGE_TOOL_KINDS,
  type ContextParameter,
  type NormalizedAgent,
  type NormalizedTool,
  type NxAgentDeclarationName,
  type ToolAnnotations,
} from "./definition.js";
import type { NxAgentDiagnostic, NxAgentDiagnosticCode, NxAgentTextSpan } from "./diagnostics.js";
import type { Agent, Tool } from "./generated/index.js";
import { parseHttpBaseUrl, parseHttpPath } from "./http.js";
import { describeValue, isJsonObject, type JsonObject, type JsonSchema, type JsonValue } from "./json.js";
import { NX_AGENT_TOOL_NAME_PATTERN, isToolName, toSnakeCase } from "./names.js";

/** A declaration to ask a schema source about: a module identity, the entry module by default, and a name. */
export interface AgentDeclarationRef {
  readonly module?: string;
  readonly name: string;
}

/** One declared parameter of a function, as far as the package reads it. */
export interface AgentParameterSchema {
  readonly name: string;
  /**
   * The record or union the parameter is declared with, by module identity and name. For a
   * parameter declared through a type alias, what the alias denotes.
   */
  readonly typeRef?: NxAgentDeclarationName;
  /** The listed host-supplied type the parameter matched. Such a parameter is not in the input schema. */
  readonly hostSupplied?: NxAgentDeclarationName;
  /**
   * The listed host-supplied types the parameter's type holds without being one, as under `+` or
   * as a field of a record. Such a parameter is in the input schema, and the package refuses it.
   */
  readonly hostSuppliedWithin?: readonly NxAgentDeclarationName[];
}

/**
 * What the package reads of a function's schema: the fields of the declaration schema export's
 * answer that it uses. An answer with more fields is one of these.
 */
export interface AgentFunctionSchema {
  /** The function's documentation, which is the tool's default description. */
  readonly description?: string;
  readonly parameters: readonly AgentParameterSchema[];
  /** Absent, with diagnostics, when a parameter's type has no JSON form. */
  readonly inputSchema?: JsonSchema;
  /** Absent, with diagnostics, when the result type has no JSON form. */
  readonly outputSchema?: JsonSchema;
  /** The span of the function's declaration in its module's source, when the source is held. */
  readonly declaration?: NxAgentTextSpan;
  /** Why a schema is absent. */
  readonly diagnostics: readonly { readonly message: string }[];
}

/**
 * Where `normalizeAgent` gets function schemas: anything with the `functionSchema` query of the
 * declaration schema export. A program artifact of `@nx-lang/sdk-wasm` or `@nx-lang/sdk-node` is
 * one as it stands. A host that asked earlier and no longer holds the artifact passes an object
 * that answers from what it kept.
 *
 * <para>An unknown function is reported by throwing an error whose `diagnostics` hold one with the
 * code `schema-unknown-declaration`, as the artifact does. Anything else thrown is not a property
 * of the agent and is thrown on by `normalizeAgent`.</para>
 */
export interface AgentSchemaSource {
  functionSchema(
    reference: AgentDeclarationRef,
    options?: { readonly hostSuppliedTypes?: readonly AgentDeclarationRef[] },
  ): AgentFunctionSchema;
}

/** One model-facing tool, as a host describer returns it for a `Tool` subtype of its own. */
export interface ToolDescription {
  readonly name: string;
  readonly title?: string;
  readonly description: string;
  readonly inputSchema: JsonSchema;
  readonly outputSchema?: JsonSchema;
  readonly annotations: ToolAnnotations;
  /** The host's kind, which names the executor that runs the tool. Not `function`, `http` or `provider`. */
  readonly kind: string;
  /** What the executor needs, as plain JSON. */
  readonly config: JsonObject;
}

/** What a describer is given beside the tool value. */
export interface ToolDescriberContext {
  readonly schemas: AgentSchemaSource;
  readonly toSnakeCase: (name: string) => string;
  /** Where the tool value is inside the agent, such as `tools[1]`. */
  readonly path: string;
}

/**
 * Describes one tool value of a host's own `Tool` subtype: as one tool, as several (the value
 * resolves to more than one model-facing tool, in the order returned), or as the problems that
 * keep it from being described.
 */
export type ToolDescriber = (
  value: Tool,
  context: ToolDescriberContext,
) => ToolDescription | readonly ToolDescription[] | { readonly diagnostics: readonly { readonly message: string }[] };

/** The options of `normalizeAgent`. A member that is `undefined` is one that is absent. */
export interface NormalizeAgentOptions {
  /** The program artifact, or any object with its `functionSchema(reference, options)`. */
  readonly schemas: AgentSchemaSource;
  /**
   * The host's concrete subtype of `ToolContext`, by module identity and name. A function may
   * declare a context parameter as `ToolContext` or as this type. With none named, the host
   * supplies no context, and a function that declares a context parameter is an error.
   */
  readonly toolContextType?: NxAgentDeclarationName | undefined;
  /** Describers for the host's own `Tool` subtypes, by the `$type` of the tool value. */
  readonly toolTypes?: Readonly<Record<string, ToolDescriber>> | undefined;
  /** Accepts an `http` base URL, for tests against a local server. Off by default. */
  readonly allowInsecureBaseUrl?: boolean | undefined;
}

/**
 * A definition, with the warnings found, or the diagnostics that kept one from being made. There
 * is never a partial definition: one error diagnostic and `ok` is `false`.
 */
export type NormalizeAgentResult =
  | { readonly ok: true; readonly definition: NormalizedAgent; readonly diagnostics: readonly NxAgentDiagnostic[] }
  | { readonly ok: false; readonly diagnostics: readonly NxAgentDiagnostic[] };

const SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema";

const WEB_SEARCH_DEFAULT_NAME = "web_search";
const WEB_SEARCH_DEFAULT_DESCRIPTION = "Searches the web and returns relevant results.";

/** The `$type` values the package normalizes itself. A describer cannot be registered for one. */
const PACKAGE_TOOL_TYPES: readonly string[] = ["FunctionTool", "HttpTool", "WebSearchTool"];

/** A schema of an object with no properties: the input of a tool that takes none. */
function emptyObjectSchema(): JsonSchema {
  return { $schema: SCHEMA_DIALECT, type: "object", properties: {}, additionalProperties: false };
}

/**
 * The output of every `http` tool: what the host's request function returns. `status` is required
 * and the object is open, so a host can add properties of its own.
 */
function httpOutputSchema(): JsonSchema {
  return {
    $schema: SCHEMA_DIALECT,
    type: "object",
    properties: { status: { type: "integer" }, body: {} },
    required: ["status"],
  };
}

/**
 * Whether an optional field holds nothing. The canonical encoding leaves an empty optional out;
 * `null` and the empty list are the other two spellings of the empty value a host may hand in.
 */
function isEmpty(value: unknown): boolean {
  return value === undefined || value === null || (Array.isArray(value) && value.length === 0);
}

/** Why `value` is not plain JSON, or nothing when it is. */
function jsonProblem(value: unknown, path: string, ancestors: readonly unknown[] = []): string | undefined {
  if (value === null || typeof value === "string" || typeof value === "boolean") {
    return undefined;
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? undefined : `${path} is ${String(value)}, which JSON cannot hold`;
  }
  if (typeof value !== "object") {
    return `${path} is ${value === undefined ? "undefined" : `a ${typeof value}`}, which JSON cannot hold`;
  }
  if (ancestors.includes(value)) {
    return `${path} holds itself`;
  }
  const within = [...ancestors, value];
  if (Array.isArray(value)) {
    for (const [index, item] of value.entries()) {
      const problem = jsonProblem(item, `${path}[${index}]`, within);
      if (problem !== undefined) {
        return problem;
      }
    }
    return undefined;
  }
  const prototype: unknown = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) {
    return `${path} is not a plain object`;
  }
  for (const [key, held] of Object.entries(value)) {
    const problem = jsonProblem(held, `${path}.${key}`, within);
    if (problem !== undefined) {
      return problem;
    }
  }
  return undefined;
}

/** A copy of a JSON value that shares nothing with it, so the definition is the package's alone. */
function copyJson<T extends JsonValue>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function sameDeclaration(left: NxAgentDeclarationName, right: NxAgentDeclarationName): boolean {
  return left.module === right.module && left.name === right.name;
}

/** The `{ module, name }` of a canonical `Function` record, if `value` is one. */
function functionRecord(value: unknown): NxAgentDeclarationName | undefined {
  if (!isJsonObject(value) || value["$type"] !== "Function") {
    return undefined;
  }
  const { module, name } = value;
  return typeof module === "string" && typeof name === "string" ? { module, name } : undefined;
}

/** Whether a schema source threw to say the program declares no such function. */
function isUnknownDeclaration(error: unknown): boolean {
  const diagnostics = (error as { diagnostics?: unknown } | null | undefined)?.diagnostics;
  return (
    Array.isArray(diagnostics) &&
    diagnostics.some((diagnostic) => (diagnostic as { code?: unknown } | null)?.code === "schema-unknown-declaration")
  );
}

/** One tool that normalized, and where its element is, for the duplicate-name check. */
interface PlacedTool {
  readonly tool: NormalizedTool;
  readonly path: string;
}

/** Collects the diagnostics of one `normalizeAgent` call. */
class Report {
  public readonly diagnostics: NxAgentDiagnostic[] = [];

  public add(
    severity: "error" | "warning",
    code: NxAgentDiagnosticCode,
    path: string,
    message: string,
    details: { readonly tool?: string | undefined; readonly declaration?: NxAgentDiagnostic["declaration"] | undefined } = {},
  ): void {
    this.diagnostics.push({
      severity,
      code,
      message,
      path,
      ...(details.tool === undefined ? {} : { tool: details.tool }),
      ...(details.declaration === undefined ? {} : { declaration: details.declaration }),
    });
  }

  public error(
    code: NxAgentDiagnosticCode,
    path: string,
    message: string,
    details: { readonly tool?: string | undefined; readonly declaration?: NxAgentDiagnostic["declaration"] | undefined } = {},
  ): void {
    this.add("error", code, path, message, details);
  }

  /** How many errors have been reported. A caller compares two readings to learn whether it added one. */
  public get errorCount(): number {
    return this.diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
  }
}

/** What a tool takes from the function it is built on: a `FunctionTool`'s function or an `HttpTool`'s arguments function. */
interface FunctionFacts {
  readonly reference: NxAgentDeclarationName;
  /** The tool's name, authored or derived, when it is a valid one. */
  readonly name: string | undefined;
  readonly description: string | undefined;
  readonly inputSchema: JsonSchema | undefined;
  readonly outputSchema: JsonSchema | undefined;
  readonly contextParameters: readonly ContextParameter[];
  /** Why the schema source answered no output schema, when it answered an input schema and no output schema. */
  readonly resultReasons: readonly { readonly message: string }[];
  /** The function's module and declaration span, for a diagnostic about it. */
  readonly declaration: NonNullable<NxAgentDiagnostic["declaration"]>;
}

/**
 * Resolves a tool's name: the authored `name` verbatim, or `fallback` otherwise. A name that does
 * not match the pattern is reported and not converted, so the name in the config is the name the
 * model sees.
 */
function resolveName(
  report: Report,
  value: JsonObject,
  path: string,
  fallback: string,
  declaration?: NxAgentDiagnostic["declaration"],
): string | undefined {
  const authored = value["name"];
  if (!isEmpty(authored) && typeof authored !== "string") {
    report.error("nx-agent-invalid-tool-name", `${path}.name`, `The name of ${path} is ${describeValue(authored)}, not a string.`);
    return undefined;
  }
  const isAuthored = typeof authored === "string";
  const name = isAuthored ? authored : fallback;
  if (!isToolName(name)) {
    report.error(
      "nx-agent-invalid-tool-name",
      isAuthored ? `${path}.name` : path,
      isAuthored
        ? `The tool name ${JSON.stringify(name)} of ${path} does not match ${NX_AGENT_TOOL_NAME_PATTERN.source}.`
        : `The name of ${path} defaults to ${JSON.stringify(name)}, which does not match ${NX_AGENT_TOOL_NAME_PATTERN.source}; give the tool a name.`,
      isAuthored ? {} : { declaration },
    );
    return undefined;
  }
  return name;
}

/**
 * Resolves a tool's description: the authored `description`, or `fallback` otherwise. One that is
 * missing, or empty once white space is trimmed, is reported.
 */
function resolveDescription(
  report: Report,
  value: JsonObject,
  path: string,
  name: string | undefined,
  fallback: string | undefined,
  declaration?: NxAgentDiagnostic["declaration"],
): string | undefined {
  const authored = value["description"];
  const isAuthored = typeof authored === "string";
  const description = isAuthored ? authored : isEmpty(authored) ? fallback : undefined;
  if (typeof description === "string" && description.trim() !== "") {
    return description;
  }
  const subject = name === undefined ? path : `Tool '${name}' (${path})`;
  report.error(
    "nx-agent-missing-description",
    isAuthored ? `${path}.description` : path,
    isAuthored
      ? `${subject} has an empty description.`
      : declaration === undefined
        ? `${subject} has no description.`
        : `${subject} has no description: give it one, or document its function with a '///' comment.`,
    { tool: name, ...(isAuthored ? {} : { declaration }) },
  );
  return undefined;
}

/**
 * Reads what a tool takes from its function: the schemas, the default name and description, and
 * the parameters the host fills in. Reports every problem it finds and answers nothing only when
 * there is no function to read.
 */
function readFunction(
  report: Report,
  options: NormalizeAgentOptions,
  value: JsonObject,
  path: string,
  field: "function" | "arguments",
): FunctionFacts | undefined {
  const reference = functionRecord(value[field]);
  if (reference === undefined) {
    report.error(
      field === "function" ? "nx-agent-unknown-function" : "nx-agent-http-arguments-type",
      `${path}.${field}`,
      `The '${field}' of ${path} is ${describeValue(value[field])}, not a function: { $type: "Function", module, name }.`,
    );
    return undefined;
  }

  let answer: AgentFunctionSchema;
  try {
    answer = options.schemas.functionSchema(reference, { hostSuppliedTypes: [NX_AGENT_TOOL_CONTEXT] });
  } catch (error) {
    if (!isUnknownDeclaration(error)) {
      // A disposed artifact or a crashed compiler says nothing about the agent.
      throw error;
    }
    report.error(
      "nx-agent-unknown-function",
      `${path}.${field}`,
      `${path} names the function '${reference.name}' of module '${reference.module}', which the program does not declare.`,
      { declaration: { module: reference.module } },
    );
    return undefined;
  }
  const declaration = {
    module: reference.module,
    ...(answer.declaration === undefined ? {} : { span: answer.declaration }),
  };

  const name = resolveName(report, value, path, toSnakeCase(reference.name), declaration);
  const description = resolveDescription(report, value, path, name, answer.description, declaration);
  const about = { tool: name, declaration };
  const subject = name === undefined ? path : `Tool '${name}' (${path})`;

  const contextTypes = [NX_AGENT_TOOL_CONTEXT, ...(options.toolContextType === undefined ? [] : [options.toolContextType])];
  const contextParameters: ContextParameter[] = [];
  for (const parameter of answer.parameters) {
    // What a parameter's type is, and what it holds, is the schema source's to say: the package
    // reads `hostSupplied`, `hostSuppliedWithin` and `typeRef`, and never the type's spelling.
    const type = parameter.typeRef;
    const isContextType = type !== undefined && contextTypes.some((candidate) => sameDeclaration(candidate, type));
    if (parameter.hostSupplied === undefined) {
      if (parameter.hostSuppliedWithin !== undefined && parameter.hostSuppliedWithin.length > 0) {
        // A list of contexts, or a record that holds one: the host cannot fill part of a value,
        // so the context's own fields would be asked of the model.
        report.error(
          "nx-agent-context-parameter",
          path,
          `${subject}: the parameter '${parameter.name}' of '${reference.name}' holds a tool context without being one, so the model would be asked for it. Declare the context as a parameter of its own.`,
          about,
        );
      }
      continue;
    }
    if (options.toolContextType === undefined) {
      report.error(
        "nx-agent-context-parameter",
        path,
        `${subject}: the parameter '${parameter.name}' of '${reference.name}' is a tool context, and the host named no tool context type, so it supplies none.`,
        about,
      );
    } else if (type === undefined || !isContextType) {
      report.error(
        "nx-agent-context-parameter",
        path,
        `${subject}: the parameter '${parameter.name}' of '${reference.name}' is declared with ${type === undefined ? "a tool context type the schema source does not name" : `'${type.name}' of '${type.module}'`}, and the host supplies '${options.toolContextType.name}' of '${options.toolContextType.module}'.`,
        about,
      );
    } else {
      contextParameters.push({ name: parameter.name, type: { module: type.module, name: type.name } });
    }
  }

  if (answer.inputSchema === undefined) {
    // The answer does not say which of its diagnostics are about a parameter. When the result has
    // no JSON form either, its diagnostic is among them and is reported under this code as well;
    // the message says which it is about, and the tool is refused either way.
    const reasons = answer.diagnostics.length === 0 ? [{ message: "a parameter's type has no JSON form" }] : answer.diagnostics;
    for (const reason of reasons) {
      report.error("nx-agent-inexpressible-parameter", path, `${subject}: ${reason.message}`, about);
    }
  }

  return {
    reference,
    name,
    description,
    inputSchema: answer.inputSchema === undefined ? undefined : copyJson(answer.inputSchema),
    outputSchema: answer.outputSchema === undefined ? undefined : copyJson(answer.outputSchema),
    contextParameters,
    // With the input schema present, the answer's diagnostics are the result's alone.
    resultReasons: answer.inputSchema !== undefined && answer.outputSchema === undefined ? answer.diagnostics : [],
    declaration,
  };
}

function normalizeFunctionTool(report: Report, options: NormalizeAgentOptions, value: JsonObject, path: string): NormalizedTool | undefined {
  const errorsBefore = report.errorCount;
  const facts = readFunction(report, options, value, path, "function");
  if (facts === undefined) {
    return undefined;
  }
  if (facts.inputSchema !== undefined && facts.outputSchema === undefined) {
    // A tool without an output schema still runs: the model reads the result as it is.
    const reasons = facts.resultReasons.length === 0 ? [{ message: "the result type has no JSON form" }] : facts.resultReasons;
    const subject = facts.name === undefined ? path : `Tool '${facts.name}' (${path})`;
    for (const reason of reasons) {
      report.add("warning", "nx-agent-inexpressible-result", path, `${subject} has no output schema: ${reason.message}`, {
        tool: facts.name,
        declaration: facts.declaration,
      });
    }
  }
  if (report.errorCount > errorsBefore || facts.name === undefined || facts.description === undefined || facts.inputSchema === undefined) {
    return undefined;
  }
  return {
    name: facts.name,
    description: facts.description,
    inputSchema: facts.inputSchema,
    ...(facts.outputSchema === undefined ? {} : { outputSchema: facts.outputSchema }),
    annotations: FUNCTION_TOOL_ANNOTATIONS,
    kind: "function",
    config: { function: facts.reference, contextParameters: facts.contextParameters },
  };
}

function normalizeWebSearchTool(report: Report, value: JsonObject, path: string): NormalizedTool | undefined {
  const errorsBefore = report.errorCount;
  const name = resolveName(report, value, path, WEB_SEARCH_DEFAULT_NAME);
  const description = resolveDescription(report, value, path, name, WEB_SEARCH_DEFAULT_DESCRIPTION);
  const domains = value["allowedDomains"];
  if (!isEmpty(domains) && !(Array.isArray(domains) && domains.every((domain) => typeof domain === "string"))) {
    report.error("nx-agent-unknown-tool-type", `${path}.allowedDomains`, `${path} is not a well-formed WebSearchTool: 'allowedDomains' is not a list of strings.`, {
      tool: name,
    });
  }
  if (report.errorCount > errorsBefore || name === undefined || description === undefined) {
    return undefined;
  }
  return {
    name,
    description,
    inputSchema: emptyObjectSchema(),
    annotations: WEB_SEARCH_TOOL_ANNOTATIONS,
    kind: "provider",
    config: {
      provider: "web_search",
      ...(isEmpty(domains) ? {} : { allowedDomains: [...(domains as string[])] }),
    },
  };
}

function normalizeHttpTool(report: Report, options: NormalizeAgentOptions, value: JsonObject, path: string): NormalizedTool | undefined {
  const errorsBefore = report.errorCount;
  // The function is read first, so every other diagnostic of the tool can carry its name.
  const facts = readFunction(report, options, value, path, "arguments");
  const tool = facts?.name;
  const subject = tool === undefined ? path : `Tool '${tool}' (${path})`;

  const connection = value["connection"];
  let connectionName: string | undefined;
  let baseUrl: string | undefined;
  if (!isJsonObject(connection) || typeof connection["name"] !== "string" || typeof connection["baseUrl"] !== "string") {
    report.error(
      "nx-agent-http-base-url",
      `${path}.connection`,
      `${subject}: its connection is ${describeValue(connection)}, not an HttpConnection with a 'name' and a 'baseUrl'.`,
      { tool },
    );
  } else {
    connectionName = connection["name"];
    const base = parseHttpBaseUrl(connection["baseUrl"], { allowInsecure: options.allowInsecureBaseUrl === true });
    if (base.ok) {
      baseUrl = base.prefix;
    } else {
      report.error(
        "nx-agent-http-base-url",
        `${path}.connection.baseUrl`,
        `${subject}: the base URL '${connection["baseUrl"]}' of connection '${connectionName}' cannot be used: ${base.problem}.`,
        { tool },
      );
    }
  }

  const method = value["method"];
  if (!isHttpToolMethod(method)) {
    report.error(
      "nx-agent-unknown-tool-type",
      `${path}.method`,
      `${subject} is not a well-formed HttpTool: its method is ${describeValue(method)}, not one of get, post, put, patch and delete.`,
      { tool },
    );
  }

  const pathTemplate = value["path"];
  let pathPlaceholders: readonly string[] | undefined;
  if (typeof pathTemplate !== "string") {
    report.error("nx-agent-http-path", `${path}.path`, `${subject}: its path is ${describeValue(pathTemplate)}, not a string.`, { tool });
  } else {
    const template = parseHttpPath(pathTemplate);
    if (template.ok) {
      pathPlaceholders = template.placeholders;
    } else {
      report.error("nx-agent-http-path", `${path}.path`, `${subject}: the path '${pathTemplate}' cannot be used: ${template.problem}.`, { tool });
    }
  }

  const maxCalls = value["maxCallsPerConversation"];
  if (!isEmpty(maxCalls) && typeof maxCalls !== "number") {
    report.error(
      "nx-agent-unknown-tool-type",
      `${path}.maxCallsPerConversation`,
      `${subject} is not a well-formed HttpTool: 'maxCallsPerConversation' is ${describeValue(maxCalls)}, not a number.`,
      { tool },
    );
  }

  if (
    report.errorCount > errorsBefore ||
    facts === undefined ||
    facts.name === undefined ||
    facts.description === undefined ||
    facts.inputSchema === undefined ||
    connectionName === undefined ||
    baseUrl === undefined ||
    !isHttpToolMethod(method) ||
    typeof pathTemplate !== "string" ||
    pathPlaceholders === undefined
  ) {
    return undefined;
  }
  return {
    name: facts.name,
    description: facts.description,
    inputSchema: facts.inputSchema,
    outputSchema: httpOutputSchema(),
    annotations: httpToolAnnotations(method),
    kind: "http",
    config: {
      arguments: facts.reference,
      contextParameters: facts.contextParameters,
      connection: { name: connectionName, baseUrl },
      method,
      path: pathTemplate,
      pathPlaceholders: [...pathPlaceholders],
      ...(typeof maxCalls === "number" ? { maxCallsPerConversation: maxCalls } : {}),
    },
  };
}

/** Why what a describer returned for one tool cannot be used, or nothing when it can. */
function descriptionProblem(description: unknown): string | undefined {
  if (!isJsonObject(description)) {
    return `it returned ${describeValue(description)}, not a tool description`;
  }
  const { name, title, kind, config, inputSchema, outputSchema, annotations } = description as { readonly [key: string]: unknown };
  if (typeof name !== "string") {
    return "the description has no 'name'";
  }
  if (title !== undefined && typeof title !== "string") {
    return `the 'title' of '${name}' is not a string`;
  }
  if (typeof kind !== "string" || kind === "") {
    return `'${name}' has no 'kind'`;
  }
  if (PACKAGE_TOOL_KINDS.includes(kind)) {
    return `'${name}' has the kind '${kind}', which is the package's own`;
  }
  if (!isJsonObject(inputSchema)) {
    return `'${name}' has no 'inputSchema'`;
  }
  if (outputSchema !== undefined && !isJsonObject(outputSchema)) {
    return `the 'outputSchema' of '${name}' is not a schema`;
  }
  if (!isJsonObject(config)) {
    return `'${name}' has no 'config'`;
  }
  const hints = ["readOnlyHint", "destructiveHint", "idempotentHint", "openWorldHint"] as const;
  if (!isJsonObject(annotations) || hints.some((hint) => typeof annotations[hint] !== "boolean")) {
    return `'${name}' does not carry the four boolean annotations`;
  }
  return (
    jsonProblem(inputSchema, `the 'inputSchema' of '${name}'`) ??
    jsonProblem(outputSchema ?? null, `the 'outputSchema' of '${name}'`) ??
    jsonProblem(config, `the 'config' of '${name}'`)
  );
}

function describeHostTool(
  report: Report,
  options: NormalizeAgentOptions,
  describer: ToolDescriber,
  value: JsonObject,
  type: string,
  path: string,
): readonly NormalizedTool[] {
  // What the describer throws is the host's own failure and is thrown on.
  const returned = describer(value as unknown as Tool, { schemas: options.schemas, toSnakeCase, path });
  if (isJsonObject(returned) && Array.isArray((returned as { diagnostics?: unknown }).diagnostics) && !("name" in returned)) {
    const diagnostics = (returned as { diagnostics: readonly unknown[] }).diagnostics;
    for (const diagnostic of diagnostics.length === 0 ? [{ message: "it reported no reason" }] : diagnostics) {
      const message = (diagnostic as { message?: unknown } | null)?.message;
      report.error("nx-agent-describer", path, `The describer of '${type}' refused ${path}: ${typeof message === "string" ? message : String(message)}`);
    }
    return [];
  }
  const descriptions: readonly unknown[] = Array.isArray(returned) ? returned : [returned];
  if (descriptions.length === 0) {
    report.error("nx-agent-describer", path, `The describer of '${type}' described no tool for ${path}.`);
    return [];
  }

  const tools: NormalizedTool[] = [];
  const errorsBefore = report.errorCount;
  for (const candidate of descriptions) {
    const problem = descriptionProblem(candidate);
    if (problem !== undefined) {
      report.error("nx-agent-describer", path, `The describer of '${type}' cannot be used for ${path}: ${problem}.`);
      continue;
    }
    const description = candidate as ToolDescription;
    // The authored fields override the defaults of a tool value that is one tool. One that is
    // several has no single name or description to override.
    const authored = descriptions.length === 1 ? value : {};
    const name = resolveName(report, authored, path, description.name);
    const text = resolveDescription(report, authored, path, name, description.description);
    if (name === undefined || text === undefined) {
      continue;
    }
    tools.push({
      name,
      ...(description.title === undefined ? {} : { title: description.title }),
      description: text,
      inputSchema: copyJson(description.inputSchema),
      ...(description.outputSchema === undefined ? {} : { outputSchema: copyJson(description.outputSchema) }),
      annotations: {
        readOnlyHint: description.annotations.readOnlyHint,
        destructiveHint: description.annotations.destructiveHint,
        idempotentHint: description.annotations.idempotentHint,
        openWorldHint: description.annotations.openWorldHint,
      },
      kind: description.kind,
      config: copyJson(description.config),
    });
  }
  return report.errorCount > errorsBefore ? [] : tools;
}

/** The value of a required string field, reported when it is not one. */
function requiredString(report: Report, value: JsonObject, field: string, path: string, owner: string): string | undefined {
  const held = value[field];
  if (typeof held === "string") {
    return held;
  }
  report.error("nx-agent-not-an-agent", path, `The value is not a well-formed ${owner}: '${field}' is ${held === undefined ? "missing" : describeValue(held)}, not a string.`);
  return undefined;
}

/** The value of an optional string field, reported when it holds something else. */
function optionalString(report: Report, value: JsonObject, field: string): string | undefined {
  const held = value[field];
  if (isEmpty(held)) {
    return undefined;
  }
  if (typeof held === "string") {
    return held;
  }
  report.error("nx-agent-not-an-agent", field, `The value is not a well-formed Agent: '${field}' is ${describeValue(held)}, not a string.`);
  return undefined;
}

/** The items of an optional list field, reported when it is not a list. */
function optionalList(report: Report, value: JsonObject, field: string): readonly unknown[] {
  const held = value[field];
  if (isEmpty(held)) {
    return [];
  }
  if (Array.isArray(held)) {
    return held;
  }
  report.error("nx-agent-not-an-agent", field, `The value is not a well-formed Agent: '${field}' is ${describeValue(held)}, not a list.`);
  return [];
}

/**
 * Turns an evaluated `Agent` into a definition a host stores, or into the diagnostics that say why
 * it cannot be one.
 *
 * <para>`agentValue` is the canonical value of an `@nx/agent` `Agent` record, as
 * `@nx-lang/ir-runtime` renders it. Each tool becomes one definition in the MCP tool shape, or
 * several when a host describer returns several, in authored order. A `FunctionTool` and an
 * `HttpTool` take their schemas, and their default name and description, from the function they
 * name, through `options.schemas`.</para>
 *
 * <para>Every problem that can be found is reported in one pass. With an error there is no
 * definition; with warnings alone there is one. The definition is plain JSON that shares nothing
 * with the value or the schemas, and the same value and schemas give the same definition, key for
 * key.</para>
 *
 * @throws whatever `options.schemas` or a describer throws, other than the schema source's report
 * of an unknown function, which is a diagnostic.
 */
export function normalizeAgent(agentValue: Agent, options: NormalizeAgentOptions): NormalizeAgentResult {
  const report = new Report();
  const value: unknown = agentValue;
  if (!isJsonObject(value) || value["$type"] !== "Agent") {
    report.error("nx-agent-not-an-agent", "", `The value is ${describeValue(value)}, not an 'Agent'.`);
    return { ok: false, diagnostics: report.diagnostics };
  }

  for (const type of Object.keys(options.toolTypes ?? {})) {
    if (PACKAGE_TOOL_TYPES.includes(type)) {
      report.error("nx-agent-describer", "", `A describer is registered for '${type}', which the package describes itself.`);
    }
  }

  const name = requiredString(report, value, "name", "name", "Agent");
  const description = optionalString(report, value, "description");
  const model = optionalString(report, value, "model");
  const instructions = requiredString(report, value, "instructions", "instructions", "Agent");

  const documents: { title: string; text: string }[] = [];
  const titles = new Map<string, number>();
  for (const [index, document] of optionalList(report, value, "documents").entries()) {
    const path = `documents[${index}]`;
    if (!isJsonObject(document)) {
      report.error("nx-agent-not-an-agent", path, `The value is not a well-formed Agent: ${path} is ${describeValue(document)}, not a Document.`);
      continue;
    }
    const title = requiredString(report, document, "title", `${path}.title`, "Document");
    const text = requiredString(report, document, "text", `${path}.text`, "Document");
    if (title === undefined || text === undefined) {
      continue;
    }
    const first = titles.get(title);
    if (first === undefined) {
      titles.set(title, index);
    } else {
      report.error(
        "nx-agent-duplicate-document-title",
        `${path}.title`,
        `The document title ${JSON.stringify(title)} is used by documents[${first}] and ${path}; each title names one document.`,
      );
    }
    documents.push({ title, text });
  }

  const limits: { maxSteps?: number; maxToolCalls?: number } = {};
  const authoredLimits = value["limits"];
  if (isJsonObject(authoredLimits)) {
    for (const field of ["maxSteps", "maxToolCalls"] as const) {
      const held = authoredLimits[field];
      if (typeof held === "number") {
        limits[field] = held;
      } else if (!isEmpty(held)) {
        report.error("nx-agent-not-an-agent", `limits.${field}`, `The value is not a well-formed Agent: 'limits.${field}' is ${describeValue(held)}, not a number.`);
      }
    }
  } else if (!isEmpty(authoredLimits)) {
    report.error("nx-agent-not-an-agent", "limits", `The value is not a well-formed Agent: 'limits' is ${describeValue(authoredLimits)}, not an AgentLimits.`);
  }

  const placed: PlacedTool[] = [];
  for (const [index, tool] of optionalList(report, value, "tools").entries()) {
    const path = `tools[${index}]`;
    const type = isJsonObject(tool) ? tool["$type"] : undefined;
    if (!isJsonObject(tool) || typeof type !== "string") {
      report.error("nx-agent-unknown-tool-type", path, `${path} is ${describeValue(tool)}, not a tool.`);
      continue;
    }
    let normalized: readonly (NormalizedTool | undefined)[];
    if (type === "FunctionTool") {
      normalized = [normalizeFunctionTool(report, options, tool, path)];
    } else if (type === "WebSearchTool") {
      normalized = [normalizeWebSearchTool(report, tool, path)];
    } else if (type === "HttpTool") {
      normalized = [normalizeHttpTool(report, options, tool, path)];
    } else {
      const describer = options.toolTypes !== undefined && Object.hasOwn(options.toolTypes, type) ? options.toolTypes[type] : undefined;
      if (describer === undefined) {
        report.error("nx-agent-unknown-tool-type", path, `${path} is a '${type}', and no describer is registered for that tool type.`);
        continue;
      }
      normalized = describeHostTool(report, options, describer, tool, type, path);
    }
    for (const one of normalized) {
      if (one !== undefined) {
        placed.push({ tool: one, path });
      }
    }
  }

  // Names are compared across every tool that has one, whichever way it came by it.
  const byName = new Map<string, string[]>();
  for (const { tool, path } of placed) {
    byName.set(tool.name, [...(byName.get(tool.name) ?? []), path]);
  }
  for (const [toolName, paths] of byName) {
    if (paths.length > 1) {
      report.error(
        "nx-agent-duplicate-tool-name",
        paths[paths.length - 1]!,
        `The tool name '${toolName}' is used by ${paths.slice(0, -1).join(", ")} and ${paths[paths.length - 1]}; each tool of an agent needs its own name.`,
        { tool: toolName },
      );
    }
  }

  if (report.errorCount > 0 || name === undefined || instructions === undefined) {
    return { ok: false, diagnostics: report.diagnostics };
  }
  const definition: NormalizedAgent = {
    formatVersion: AGENT_DEFINITION_FORMAT_VERSION,
    name,
    ...(description === undefined ? {} : { description }),
    ...(model === undefined ? {} : { model }),
    instructions,
    documents,
    limits,
    tools: placed.map(({ tool }) => tool),
  };
  return { ok: true, definition, diagnostics: report.diagnostics };
}
