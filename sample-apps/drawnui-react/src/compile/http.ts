import type { Compile, CompileResult } from "./types";

/**
 * How long a compile may take before the app gives up on it.
 *
 * A stalled request is worse than a failed one: the editor keeps working either way, but a session
 * stuck on "Compiling…" gives no reason to reload, so a transport that never answers is turned into
 * a reported failure.
 */
const TIMEOUT_MS = 8000;

/** Compiles by asking the app's own server. */
export const compileOverHttp: Compile = async (source: string): Promise<CompileResult> => {
  let response: Response;
  try {
    response = await fetch("/api/compile", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source }),
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
  } catch (error) {
    const reason = error instanceof Error && error.name === "TimeoutError"
      ? `it did not answer within ${TIMEOUT_MS / 1000}s`
      : String(error);
    throw new Error(`Could not reach the compile service: ${reason}.`);
  }
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`compile failed (${response.status}): ${detail.slice(0, 200)}`);
  }
  return (await response.json()) as CompileResult;
};
