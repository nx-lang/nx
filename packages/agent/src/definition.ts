import type { JsonObject, JsonSchema } from "./json.js";

/**
 * The version of the stored definition format this package writes and reads. While the package is
 * unstable the version may change in any release; `createAgentTools` refuses one it does not know.
 */
export const AGENT_DEFINITION_FORMAT_VERSION = 1;

/** A declaration by the identity of the module that declares it and its name. */
export interface NxAgentDeclarationName {
  readonly module: string;
  readonly name: string;
}

/**
 * The abstract `ToolContext` of the `@nx/agent` library: the type `normalizeAgent` lists as
 * host-supplied when it asks for a function's schema. A host that asks for schemas itself lists
 * the same reference.
 */
export const NX_AGENT_TOOL_CONTEXT: NxAgentDeclarationName = Object.freeze({
  module: "@nx/agent/agent.nx",
  name: "ToolContext",
});

/** The four MCP tool hints. The package sets them from a tool's type; an author never does. */
export interface ToolAnnotations {
  readonly readOnlyHint: boolean;
  readonly destructiveHint: boolean;
  readonly idempotentHint: boolean;
  readonly openWorldHint: boolean;
}

/** What every normalized tool carries: the MCP tool shape. */
export interface NormalizedToolBase {
  readonly name: string;
  readonly title?: string;
  readonly description: string;
  /** JSON Schema draft 2020-12, always of an object. */
  readonly inputSchema: JsonSchema;
  /** JSON Schema draft 2020-12 of the output as it is, of any JSON type. */
  readonly outputSchema?: JsonSchema;
  readonly annotations: ToolAnnotations;
}

/** A function parameter the host fills in: its name and its declared type. */
export interface ContextParameter {
  readonly name: string;
  readonly type: NxAgentDeclarationName;
}

export interface FunctionToolConfig {
  readonly function: NxAgentDeclarationName;
  readonly contextParameters: readonly ContextParameter[];
}

export type HttpToolMethod = "get" | "post" | "put" | "patch" | "delete";

export interface HttpToolConfig {
  /** The function that returns the call's `HttpArguments`. */
  readonly arguments: NxAgentDeclarationName;
  readonly contextParameters: readonly ContextParameter[];
  /** The connection, its base URL without a trailing `/`. */
  readonly connection: { readonly name: string; readonly baseUrl: string };
  readonly method: HttpToolMethod;
  /** The path template, which begins with `/` and may hold `{name}` placeholders. */
  readonly path: string;
  /** The placeholder names of `path`, each once, in the order they first appear. */
  readonly pathPlaceholders: readonly string[];
  readonly maxCallsPerConversation?: number;
}

export interface ProviderToolConfig {
  readonly provider: "web_search";
  readonly allowedDomains?: readonly string[];
}

export type NormalizedFunctionTool = NormalizedToolBase & {
  readonly kind: "function";
  readonly config: FunctionToolConfig;
};

export type NormalizedHttpTool = NormalizedToolBase & {
  readonly kind: "http";
  readonly config: HttpToolConfig;
};

export type NormalizedProviderTool = NormalizedToolBase & {
  readonly kind: "provider";
  readonly config: ProviderToolConfig;
};

/** A tool of a kind a host describer supplied, with the `config` it stored. */
export type NormalizedHostTool = NormalizedToolBase & {
  readonly kind: string;
  readonly config: JsonObject;
};

/**
 * One tool of a stored definition. `kind` is `function`, `http` or `provider` for the package's
 * own tools, and any other string for a host's. `config` holds what the tool's executor needs.
 */
export type NormalizedTool = NormalizedFunctionTool | NormalizedHttpTool | NormalizedProviderTool | NormalizedHostTool;

/**
 * An evaluated `Agent` as a host stores it: plain JSON, with every tool in the MCP tool shape. An
 * optional field with no value is absent, never `null`.
 */
export interface NormalizedAgent {
  readonly formatVersion: typeof AGENT_DEFINITION_FORMAT_VERSION;
  readonly name: string;
  readonly description?: string;
  /** As authored. Resolving it is the host's. */
  readonly model?: string;
  readonly instructions: string;
  readonly documents: readonly { readonly title: string; readonly text: string }[];
  /** As authored. Capping them is the host's. */
  readonly limits: { readonly maxSteps?: number; readonly maxToolCalls?: number };
  readonly tools: readonly NormalizedTool[];
}

/** The kinds the package executes or describes itself, which a host cannot register. */
export const PACKAGE_TOOL_KINDS: readonly string[] = ["function", "http", "provider"];

export function isFunctionTool(tool: NormalizedTool): tool is NormalizedFunctionTool {
  return tool.kind === "function";
}

export function isHttpTool(tool: NormalizedTool): tool is NormalizedHttpTool {
  return tool.kind === "http";
}

export function isProviderTool(tool: NormalizedTool): tool is NormalizedProviderTool {
  return tool.kind === "provider";
}
