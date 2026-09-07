import assert from "node:assert/strict";
import { test } from "node:test";
import type { HoverRequest } from "@nx-lang/language-protocol";
import {
  LanguageServiceHttpError,
  LanguageServiceTimeoutError,
  UnsupportedQueryError,
  createHttpLanguageService,
  isAbortError,
} from "../src/index.js";

const FORM = "nx://tenant/form.nx";
const hoverRequest: HoverRequest = {
  documents: [{ uri: FORM, source: "let value = 1\n", version: 1 }],
  uri: FORM,
  position: { line: 0, character: 5 },
};

interface Call {
  url: string;
  init: RequestInit;
}

/** A fetch that records what it was asked and answers with `respond`. */
function fakeFetch(respond: (call: Call) => Response | Promise<Response>): { fetch: typeof fetch; calls: Call[] } {
  const calls: Call[] = [];
  const impl = (async (input: string | URL | Request, init?: RequestInit) => {
    const call = { url: String(input), init: init ?? {} };
    calls.push(call);
    return respond(call);
  }) as typeof fetch;
  return { fetch: impl, calls };
}

test("posts JSON to <baseUrl>/<query> and parses the answer", async () => {
  const { fetch, calls } = fakeFetch(() => Response.json(null));
  const service = createHttpLanguageService({ baseUrl: "https://api.example.test/nx/language/", fetch });

  const answer = await service.hover(hoverRequest);

  assert.equal(answer, null);
  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.url, "https://api.example.test/nx/language/hover");
  assert.equal(calls[0]!.init.method, "POST");
  assert.equal(new Headers(calls[0]!.init.headers).get("content-type"), "application/json");
  assert.deepEqual(JSON.parse(calls[0]!.init.body as string), hoverRequest);
});

test("every query request carries host-supplied headers, static or per request", async () => {
  const staticFetch = fakeFetch(() => Response.json([]));
  const withStatic = createHttpLanguageService({
    baseUrl: "/api/language",
    fetch: staticFetch.fetch,
    headers: { authorization: "Bearer static" },
  });
  await withStatic.documentSymbols({ documents: hoverRequest.documents, uri: FORM });
  assert.equal(new Headers(staticFetch.calls[0]!.init.headers).get("authorization"), "Bearer static");

  const seen: string[] = [];
  const hookFetch = fakeFetch(() => Response.json({ documents: [], workspace: [] }));
  const withHook = createHttpLanguageService({
    baseUrl: "/api/language",
    fetch: hookFetch.fetch,
    headers: async (query) => {
      seen.push(query);
      return { authorization: `Bearer token-for-${query}` };
    },
  });
  await withHook.diagnostics({ documents: hoverRequest.documents, uri: FORM });
  assert.deepEqual(seen, ["diagnostics"]);
  assert.equal(new Headers(hookFetch.calls[0]!.init.headers).get("authorization"), "Bearer token-for-diagnostics");
});

test("a non-success status rejects with the status, code and a body excerpt", async () => {
  const body = { error: { code: "invalid-request", message: "Request is missing 'uri'. " + "x".repeat(400), query: "hover" } };
  const { fetch } = fakeFetch(() => Response.json(body, { status: 400 }));
  const service = createHttpLanguageService({ baseUrl: "/api/language", fetch });

  await assert.rejects(service.hover(hoverRequest), (error: unknown) => {
    assert.ok(error instanceof LanguageServiceHttpError);
    assert.equal(error.status, 400);
    assert.equal(error.code, "invalid-request");
    assert.equal(error.url, "/api/language/hover");
    assert.equal(error.bodyExcerpt.length, 200);
    assert.match(error.message, /400/);
    return true;
  });

  const html = fakeFetch(() => new Response("<html>gateway down</html>", { status: 502 }));
  const gateway = createHttpLanguageService({ baseUrl: "/api/language", fetch: html.fetch });
  await assert.rejects(gateway.hover(hoverRequest), (error: unknown) => {
    assert.ok(error instanceof LanguageServiceHttpError);
    assert.equal(error.status, 502);
    assert.equal(error.code, undefined);
    assert.equal(error.bodyExcerpt, "<html>gateway down</html>");
    return true;
  });
});

test("an unsupported-query body becomes UnsupportedQueryError", async () => {
  const { fetch } = fakeFetch(() =>
    Response.json({ error: { code: "unsupported-query", message: "not yet", query: "hover" } }, { status: 501 }),
  );
  const service = createHttpLanguageService({ baseUrl: "/api/language", fetch });

  await assert.rejects(service.hover(hoverRequest), (error: unknown) => {
    assert.ok(error instanceof UnsupportedQueryError);
    assert.equal(error.query, "hover");
    assert.equal(error.message, "not yet");
    return true;
  });
});

test("a stalled request times out and the underlying request is aborted", async () => {
  let aborted = false;
  const { fetch } = fakeFetch(
    ({ init }) =>
      new Promise<Response>((_, reject) => {
        init.signal!.addEventListener("abort", () => {
          aborted = true;
          reject(new DOMException("aborted", "AbortError"));
        });
      }),
  );
  const service = createHttpLanguageService({ baseUrl: "/api/language", fetch, timeoutMs: 20 });

  await assert.rejects(service.hover(hoverRequest), (error: unknown) => {
    assert.ok(error instanceof LanguageServiceTimeoutError);
    assert.equal(error.timeoutMs, 20);
    return true;
  });
  assert.ok(aborted);
});

test("cancelling the caller's signal rejects with an abort error and never resolves later", async () => {
  let resolveFetch: ((response: Response) => void) | undefined;
  const { fetch } = fakeFetch(
    ({ init }) =>
      new Promise<Response>((resolve, reject) => {
        resolveFetch = resolve;
        init.signal!.addEventListener("abort", () => reject(init.signal!.reason));
      }),
  );
  const service = createHttpLanguageService({ baseUrl: "/api/language", fetch });
  const controller = new AbortController();

  const pending = service.hover(hoverRequest, controller.signal);
  let settled: "resolved" | "rejected" | undefined;
  const observed = pending.then(
    () => (settled = "resolved"),
    (error: unknown) => {
      settled = "rejected";
      assert.ok(isAbortError(error), String(error));
    },
  );
  controller.abort();
  await observed;
  assert.equal(settled, "rejected");

  // A late answer changes nothing.
  resolveFetch?.(Response.json({ contents: "late" }));
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(settled, "rejected");

  // An already-cancelled signal never sends.
  const never = fakeFetch(() => Response.json(null));
  const cancelled = new AbortController();
  cancelled.abort();
  const early = createHttpLanguageService({ baseUrl: "/api/language", fetch: never.fetch });
  await assert.rejects(early.hover(hoverRequest, cancelled.signal), (error: unknown) => isAbortError(error));
  assert.equal(never.calls.length, 0);
});

test("an answer that arrives after cancellation is discarded", async () => {
  const controller = new AbortController();
  const { fetch } = fakeFetch(() => {
    controller.abort();
    return Response.json(null);
  });
  const service = createHttpLanguageService({ baseUrl: "/api/language", fetch });

  await assert.rejects(service.hover(hoverRequest, controller.signal), (error: unknown) => isAbortError(error));
});

test("the timeout and the caller's signal cover the header hook", async () => {
  const never = fakeFetch(() => Response.json(null));
  const timingOut = createHttpLanguageService({
    baseUrl: "/api/language",
    fetch: never.fetch,
    headers: () => new Promise<HeadersInit>(() => undefined),
    timeoutMs: 20,
  });
  await assert.rejects(timingOut.hover(hoverRequest), (error: unknown) => error instanceof LanguageServiceTimeoutError);

  const controller = new AbortController();
  const cancelled = createHttpLanguageService({
    baseUrl: "/api/language",
    fetch: never.fetch,
    headers: () => {
      queueMicrotask(() => controller.abort());
      return new Promise<HeadersInit>(() => undefined);
    },
  });
  await assert.rejects(cancelled.hover(hoverRequest, controller.signal), (error: unknown) => isAbortError(error));
  assert.equal(never.calls.length, 0, "a query stopped in its header hook never sends");
});

test("a 200 whose body never completes times out, or is abandoned when the caller cancels", async () => {
  const stalledBody = () => new Response(new ReadableStream<Uint8Array>({ start: () => undefined }), { status: 200 });

  const { fetch } = fakeFetch(stalledBody);
  const timingOut = createHttpLanguageService({ baseUrl: "/api/language", fetch, timeoutMs: 20 });
  await assert.rejects(timingOut.hover(hoverRequest), (error: unknown) => {
    assert.ok(error instanceof LanguageServiceTimeoutError);
    assert.equal(error.timeoutMs, 20);
    return true;
  });

  const controller = new AbortController();
  const cancelled = createHttpLanguageService({ baseUrl: "/api/language", fetch });
  const pending = cancelled.completions({ ...hoverRequest }, controller.signal);
  let settled: "resolved" | "rejected" | undefined;
  const observed = pending.then(
    () => (settled = "resolved"),
    (error: unknown) => {
      settled = "rejected";
      assert.ok(isAbortError(error), String(error));
    },
  );
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(settled, undefined, "the body is still being waited on");
  controller.abort();
  await observed;
  assert.equal(settled, "rejected");
});
