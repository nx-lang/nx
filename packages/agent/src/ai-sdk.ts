/**
 * The AI SDK adapter of `@nx-lang/agent`: maps executable tools to Vercel AI SDK 7 tools. This is
 * the one entry point that imports `ai`, which is an optional peer dependency.
 *
 * UNSTABLE. The API may change in any release.
 */
import { dynamicTool, jsonSchema, type ToolExecutionOptions, type ToolSet } from "ai";

import { NxAgentToolError, type ToolError, type ToolErrorCode } from "./diagnostics.js";
import type { AgentTool, ToolContextRecord, ToolResult } from "./execute.js";

/** One tool call as the AI SDK made it: what a host derives a `callId` or a tool context from. */
export interface AiSdkToolCall {
  readonly toolName: string;
  /** The provider's identifier of the call. It is not stable across a re-run of the model step. */
  readonly toolCallId: string;
  readonly input: unknown;
  /** The SDK's own options for the execution: the messages, the abort signal and the SDK's context. */
  readonly options: ToolExecutionOptions<unknown>;
}

/** What `onResult` is told about one call of an executable tool. */
export interface AiSdkToolResult {
  /** The tool's name. */
  readonly name: string;
  /** The `callId` the tool was executed with. */
  readonly callId: string;
  /** The whole result as the tool's `execute` resolved it, `usage` included. */
  readonly result: ToolResult;
}

/** The options of `toAiSdkTools`. A member that is `undefined` is one that is absent. */
export interface ToAiSdkToolsOptions {
  /**
   * Derives the `callId` of a call. Without it the `callId` is the SDK's `toolCallId`, which a
   * provider does not keep across a re-run of the model step; a host that retries supplies its own.
   */
  readonly callId?: ((call: AiSdkToolCall) => string) | undefined;
  /** Supplies the host's tool context record for a call. */
  readonly context?: ((call: AiSdkToolCall) => ToolContextRecord | undefined) | undefined;
  /**
   * Maps a `provider` tool to the model provider's own tool, such as
   * `openai.tools.webSearch(...)`, or answers `null` to leave it out of the tool set. The package
   * cannot make a provider's tool itself, and a provider tool with no mapping is an error.
   */
  readonly providerTool?: ((tool: AgentTool) => ToolSet[string] | null | undefined) | undefined;
  /**
   * Called once for every call of an executable tool, on a success and on a failure, with the
   * whole result. The SDK is given the output alone, so this is where a host reads a call's
   * `usage` and where it journals the result. What it returns is awaited before the output goes
   * to the SDK, and what it throws, or rejects with, is not caught: the SDK reports it as the
   * tool's error.
   */
  readonly onResult?: ((event: AiSdkToolResult) => void | PromiseLike<void>) | undefined;
}

/**
 * What the model is told about a failure with one of the package's own codes, other than
 * `invalid-input`: what kind of failure it was, and nothing of why. The reason is the host's to
 * read, in `onResult`, and may hold what a model should not: the text a request function threw,
 * the runtime's account of a function, a connection's address.
 */
const MODEL_FACING: Readonly<Record<Exclude<ToolErrorCode, "invalid-input">, (tool: string) => string>> = {
  "invalid-context": (tool) => `Tool '${tool}' is not available for this call.`,
  "evaluation-failed": (tool) => `Tool '${tool}' failed while it ran.`,
  "resource-limit": (tool) => `Tool '${tool}' stopped at a limit on what one call may do.`,
  "invalid-request": (tool) => `Tool '${tool}' could not make a request from these arguments.`,
  // Not "did not complete": a request that timed out may have been carried out.
  "request-failed": (tool) => `Tool '${tool}' did not get an answer to its request.`,
  aborted: (tool) => `The call of tool '${tool}' was cancelled.`,
};

/** The error the SDK is given for a failed call, which is what the model sees of it. */
function modelFacingError(tool: string, error: ToolError): NxAgentToolError {
  const sentence = Object.hasOwn(MODEL_FACING, error.code) ? MODEL_FACING[error.code as keyof typeof MODEL_FACING] : undefined;
  if (sentence === undefined) {
    // `invalid-input` is the model's to correct, so it is told what was wrong. Any other code here
    // is one a host function chose, and its message is the host's to have written for the model.
    // The SDK sends the model the error's name and message; the diagnostics and the limit ride on
    // the error for the host's own handlers.
    return new NxAgentToolError(error.code, error.message, { diagnostics: error.diagnostics, limit: error.limit });
  }
  return new NxAgentToolError(error.code, sentence(tool));
}

/**
 * Maps executable tools to an AI SDK tool set, keyed by tool name.
 *
 * <para>A tool with `execute` becomes a tool defined at run time, with the definition's description
 * and its input schema, and nothing else set: no strict mode and no approval. Its output goes to
 * the SDK as it is, and a failure is thrown as an {@link NxAgentToolError}, which the SDK turns into
 * a tool error the model sees. A `provider` tool is mapped by `options.providerTool`.</para>
 *
 * <para>The SDK sends the model the thrown error's name and message, so the message is what the
 * model is told. It is the failure's own message only when the code is `invalid-input`, which the
 * model can correct, or a code a host function chose: a host that throws an
 * {@link NxAgentToolError} with a code of its own is writing to the model, and puts nothing in
 * the message it would not show one, the text of an error it caught included. For the package's
 * other codes the message is a fixed sentence that says what kind of failure it was, and
 * `options.onResult` is where the host reads the rest. A host that wants the model told something
 * else throws its own error from `onResult`.</para>
 *
 * <para>The adapter calls no model.</para>
 *
 * @throws TypeError when a provider tool has no mapping: `options.providerTool` is absent, or
 * answers neither a tool nor `null` for it.
 */
export function toAiSdkTools(tools: readonly AgentTool[], options: ToAiSdkToolsOptions = {}): ToolSet {
  const toolSet: ToolSet = {};
  for (const tool of tools) {
    const { execute } = tool;
    if (execute === undefined) {
      const mapped = options.providerTool?.(tool);
      if (mapped === undefined) {
        throw new TypeError(
          `Tool '${tool.name}' is run by the model's provider, and 'providerTool' gave no tool for it. Map it to the provider's tool, or answer null to leave it out.`,
        );
      }
      if (mapped !== null) {
        toolSet[tool.name] = mapped;
      }
      continue;
    }
    toolSet[tool.name] = dynamicTool({
      description: tool.description,
      // The SDK types a JSON Schema as draft 7. It forwards the object to the provider and does
      // not read it, so the draft 2020-12 schema the definition holds is passed as it is.
      inputSchema: jsonSchema(tool.inputSchema as Parameters<typeof jsonSchema>[0]),
      execute: async (input, sdkOptions) => {
        const call: AiSdkToolCall = { toolName: tool.name, toolCallId: sdkOptions.toolCallId, input, options: sdkOptions };
        const callId = options.callId === undefined ? sdkOptions.toolCallId : options.callId(call);
        const context = options.context?.(call);
        const result = await execute(input, { callId, context, signal: sdkOptions.abortSignal });
        // Awaited, and not caught: a host that journals here has written the result before the
        // model goes on, and its failure to do so is the tool's error.
        await options.onResult?.({ name: tool.name, callId, result });
        if (!result.ok) {
          throw modelFacingError(tool.name, result.error);
        }
        return result.output;
      },
    });
  }
  return toolSet;
}
