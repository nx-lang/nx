/**
 * The root entry of `@nx-lang/agent`: the types of the `@nx/agent` library, the stored definition
 * format, the naming helper and the HTTP request builder. Nothing here imports anything at run
 * time, so it loads anywhere, a Cloudflare Worker included.
 *
 * UNSTABLE. The API and the stored definition format may change in any release.
 */

// The types `nxlang typegen @nx/agent` generates, under their NX names. Generated code for a host
// library that references the agent library imports them from this package.
export type * from "./generated/index.js";

export {
  AGENT_DEFINITION_FORMAT_VERSION,
  NX_AGENT_TOOL_CONTEXT,
  isFunctionTool,
  isHttpTool,
  isProviderTool,
} from "./definition.js";
export type {
  ContextParameter,
  FunctionToolConfig,
  HttpToolConfig,
  HttpToolMethod,
  NormalizedAgent,
  NormalizedFunctionTool,
  NormalizedHostTool,
  NormalizedHttpTool,
  NormalizedProviderTool,
  NormalizedTool,
  NormalizedToolBase,
  NxAgentDeclarationName,
  ProviderToolConfig,
  ToolAnnotations,
} from "./definition.js";

export { NxAgentError, NxAgentToolError } from "./diagnostics.js";
export type {
  NxAgentDiagnostic,
  NxAgentDiagnosticCode,
  NxAgentTextSpan,
  ToolError,
  ToolErrorCode,
  ToolLimit,
  ToolUsage,
} from "./diagnostics.js";

export { NX_AGENT_TOOL_NAME_PATTERN, isToolName, toSnakeCase } from "./names.js";

export { buildHttpRequest } from "./http.js";
export type {
  BuildHttpRequestResult,
  HttpArgumentsData,
  HttpParamData,
  HttpRequestDescription,
  HttpRequestError,
  HttpRequestRule,
} from "./http.js";

export type { JsonObject, JsonSchema, JsonValue } from "./json.js";
