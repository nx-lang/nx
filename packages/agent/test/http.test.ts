import assert from "node:assert/strict";
import { test } from "node:test";

import { parseHttpBaseUrl, parseHttpPath, underBaseUrl } from "../src/http.js";
import { buildHttpRequest, type BuildHttpRequestResult, type HttpRequestRule } from "../src/index.js";
import { httpArguments, httpTool } from "./http-support.js";

function built(result: BuildHttpRequestResult) {
  assert.equal(result.ok, true, result.ok ? "" : result.error.message);
  return (result as Extract<BuildHttpRequestResult, { ok: true }>).request;
}

function refused(result: BuildHttpRequestResult, rule: HttpRequestRule) {
  assert.equal(result.ok, false, "the request was built");
  const { error } = result as Extract<BuildHttpRequestResult, { ok: false }>;
  assert.equal(error.code, "invalid-request");
  assert.equal(error.rule, rule, error.message);
  assert.equal(error.tool, "lookup_order");
  assert.match(error.message, /^Tool 'lookup_order': /);
  return error;
}

// ---- static validation: a connection's base URL ------------------------------------------------

test("an https base URL is accepted, with or without a path and a trailing slash", () => {
  assert.deepEqual(parseHttpBaseUrl("https://api.example.com"), {
    ok: true,
    prefix: "https://api.example.com",
    origin: "https://api.example.com",
    path: "",
  });
  assert.deepEqual(parseHttpBaseUrl("https://api.example.com/"), {
    ok: true,
    prefix: "https://api.example.com",
    origin: "https://api.example.com",
    path: "",
  });
  assert.deepEqual(parseHttpBaseUrl("https://api.example.com:8443/v1/"), {
    ok: true,
    prefix: "https://api.example.com:8443/v1",
    origin: "https://api.example.com:8443",
    path: "/v1",
  });
  assert.equal(parseHttpBaseUrl("https://api.example.com/v1/accounts").ok, true);
});

test("a base URL that is not an absolute https URL is refused", () => {
  const problem = (baseUrl: string, options = {}) => {
    const result = parseHttpBaseUrl(baseUrl, options);
    assert.equal(result.ok, false, baseUrl);
    return (result as { problem: string }).problem;
  };
  assert.match(problem("api.example.com"), /not an absolute URL/);
  assert.match(problem("/v1"), /not an absolute URL/);
  assert.match(problem(""), /not an absolute URL/);
  assert.match(problem("http://api.example.com"), /scheme is 'http', not 'https'/);
  assert.match(problem("ftp://api.example.com"), /scheme is 'ftp'/);
  assert.match(problem("file:///etc/passwd"), /scheme is 'file'/);
  assert.match(problem("javascript:alert(1)"), /scheme is 'javascript'/);
  // The option admits http and nothing else.
  assert.match(problem("ftp://api.example.com", { allowInsecure: true }), /scheme is 'ftp'/);
});

test("the http scheme is accepted only when the host allows it", () => {
  assert.equal(parseHttpBaseUrl("http://localhost:8787").ok, false);
  assert.deepEqual(parseHttpBaseUrl("http://localhost:8787", { allowInsecure: true }), {
    ok: true,
    prefix: "http://localhost:8787",
    origin: "http://localhost:8787",
    path: "",
  });
});

test("a base URL with a query, a fragment or user information is refused", () => {
  const problem = (baseUrl: string) => (parseHttpBaseUrl(baseUrl) as { problem: string }).problem;
  assert.match(problem("https://api.example.com/v1?key=1"), /has a query/);
  assert.match(problem("https://api.example.com/v1?"), /has a query/);
  assert.match(problem("https://api.example.com/v1#top"), /has a fragment/);
  assert.match(problem("https://api.example.com/v1#"), /has a fragment/);
  assert.match(problem("https://user:secret@api.example.com"), /has user information/);
  assert.match(problem("https://user@api.example.com"), /has user information/);
  assert.match(problem("https://@api.example.com"), /has user information/);
});

test("a base URL a parser would rewrite is refused, naming the spelling to write", () => {
  const problem = (baseUrl: string) => (parseHttpBaseUrl(baseUrl) as { problem: string }).problem;
  assert.match(problem("https://API.example.com"), /reads it as 'https:\/\/api\.example\.com'/);
  assert.match(problem("https://api.example.com:443/v1"), /reads it as 'https:\/\/api\.example\.com\/v1'/);
  assert.match(problem("https://api.example.com/v1/../admin"), /reads it as 'https:\/\/api\.example\.com\/admin'/);
  assert.match(problem("https://api.example.com/a b"), /reads it as 'https:\/\/api\.example\.com\/a%20b'/);
  assert.match(problem("https:\\\\api.example.com"), /reads it as/);
});

// ---- static validation: a tool's path template -------------------------------------------------

test("a path template is read into its placeholders", () => {
  const placeholders = (path: string) => {
    const result = parseHttpPath(path);
    assert.equal(result.ok, true, path);
    return (result as { placeholders: readonly string[] }).placeholders;
  };
  assert.deepEqual(placeholders("/"), []);
  assert.deepEqual(placeholders("/orders"), []);
  assert.deepEqual(placeholders("/orders/{orderId}"), ["orderId"]);
  assert.deepEqual(placeholders("/accounts/{account_id}/orders/{orderId}/lines"), ["account_id", "orderId"]);
  assert.deepEqual(placeholders("/files/{name}.json"), ["name"]);
  assert.deepEqual(placeholders("/a/{x}/b/{x}"), ["x"]);
  assert.deepEqual(placeholders("/v1.2/it~em_s-x/a:b@c;d=e,f+g!$&'()*"), []);
  assert.deepEqual(placeholders("/caf%C3%A9/{id}"), ["id"]);
});

test("a malformed path template is refused", () => {
  const problem = (path: string) => {
    const result = parseHttpPath(path);
    assert.equal(result.ok, false, path);
    return (result as { problem: string }).problem;
  };
  assert.match(problem("orders"), /does not begin with '\/'/);
  assert.match(problem(""), /does not begin with '\/'/);
  assert.match(problem("https://evil.example/x"), /does not begin with '\/'/);
  assert.match(problem("/orders/{orderId"), /is not closed/);
  assert.match(problem("/orders/{a{b}}"), /is not closed/);
  assert.match(problem("/orders/orderId}"), /closes no placeholder/);
  assert.match(problem("/orders/{}"), /not named by an identifier/);
  assert.match(problem("/orders/{order-id}"), /not named by an identifier/);
  assert.match(problem("/orders/{1st}"), /not named by an identifier/);
  assert.match(problem("/orders/{ id }"), /not named by an identifier/);
  assert.match(problem("/orders?state=open"), /"\?" at offset 7 is not a URL path character/);
  assert.match(problem("/orders#top"), /"#" at offset 7/);
  assert.match(problem("/orders\\admin"), /is not a URL path character/);
  assert.match(problem("/my orders"), /" " at offset 3/);
  assert.match(problem("/café"), /is not a URL path character/);
  assert.match(problem("/orders/%zz"), /not followed by two hexadecimal digits/);
  assert.match(problem("/orders/%4"), /not followed by two hexadecimal digits/);
  assert.match(problem("/orders/\u0000"), /is not a URL path character/);
});

test("a literal dot segment is refused, in every spelling a parser acts on", () => {
  for (const path of ["/orders/../admin", "/orders/./x", "/..", "/.", "/orders/..", "/a/%2e%2e/b", "/a/.%2E/b", "/a/%2E/b"]) {
    const result = parseHttpPath(path);
    assert.equal(result.ok, false, path);
    assert.match((result as { problem: string }).problem, /has the segment/, path);
  }
  // A dot that is only part of a segment is an ordinary character.
  for (const path of ["/v1.2/x", "/.well-known/x", "/a/.../b", "/a/..b"]) {
    assert.equal(parseHttpPath(path).ok, true, path);
  }
  // A segment a placeholder contributes to is checked when it is filled.
  assert.equal(parseHttpPath("/files/.{name}").ok, true);
});

// ---- buildHttpRequest: the spec's scenarios ----------------------------------------------------

test("a path parameter is substituted and encoded", () => {
  const request = built(buildHttpRequest(httpTool("get", "/orders/{orderId}"), httpArguments({ orderId: "A 1/2" })));
  assert.deepEqual(request, { method: "GET", url: "https://api.example.com/orders/A%201%2F2", headers: {} });
});

test("a base path is kept, joined by exactly one slash", () => {
  const tool = httpTool("get", "/orders/{orderId}", "https://api.example.com/v1/");
  assert.equal(built(buildHttpRequest(tool, httpArguments({ orderId: "7" }))).url, "https://api.example.com/v1/orders/7");
  const stored = httpTool("get", "/orders/{orderId}", "https://api.example.com/v1");
  assert.equal(built(buildHttpRequest(stored, httpArguments({ orderId: "7" }))).url, "https://api.example.com/v1/orders/7");
});

test("query parameters are encoded in order", () => {
  const request = built(
    buildHttpRequest(httpTool("get", "/search"), httpArguments({}, [["q", "a&b=c"], ["page", "2"], ["q", "again"], ["empty", ""]])),
  );
  assert.equal(request.url, "https://api.example.com/search?q=a%26b%3Dc&page=2&q=again&empty=");
});

test("only the unreserved characters are left as they are", () => {
  const unreserved = "AZaz09-._~";
  const request = built(
    buildHttpRequest(httpTool("get", "/x/{v}"), httpArguments({ v: `${unreserved}!'()*:@$&+,;= é𝄞` }, [["k y", "!'()*"]])),
  );
  assert.equal(
    request.url,
    `https://api.example.com/x/${unreserved}%21%27%28%29%2A%3A%40%24%26%2B%2C%3B%3D%20%C3%A9%F0%9D%84%9E?k%20y=%21%27%28%29%2A`,
  );
});

test("a traversal value is refused", () => {
  const tool = httpTool("get", "/files/{name}");
  refused(buildHttpRequest(tool, httpArguments({ name: ".." })), "dot-segment");
  refused(buildHttpRequest(tool, httpArguments({ name: "." })), "dot-segment");
  // Encoded dots in a value are data: their percent signs are encoded, so no parser reads a dot.
  assert.equal(built(buildHttpRequest(tool, httpArguments({ name: "%2e%2e" }))).url, "https://api.example.com/files/%252e%252e");
  assert.equal(built(buildHttpRequest(tool, httpArguments({ name: "../../etc" }))).url, "https://api.example.com/files/..%2F..%2Fetc");
  assert.equal(built(buildHttpRequest(tool, httpArguments({ name: "..." }))).url, "https://api.example.com/files/...");
});

test("a dot segment assembled from a literal and a value is refused", () => {
  refused(buildHttpRequest(httpTool("get", "/files/.{name}"), httpArguments({ name: "." })), "dot-segment");
  refused(buildHttpRequest(httpTool("get", "/files/{a}{b}"), httpArguments({ a: ".", b: "." })), "dot-segment");
  refused(buildHttpRequest(httpTool("get", "/files/%2e{name}"), httpArguments({ name: "." })), "dot-segment");
  refused(buildHttpRequest(httpTool("get", "/files/{name}%2E/x"), httpArguments({ name: "." })), "dot-segment");
  assert.equal(
    built(buildHttpRequest(httpTool("get", "/files/.{name}"), httpArguments({ name: "env" }))).url,
    "https://api.example.com/files/.env",
  );
});

test("a missing path parameter is refused, naming the placeholder", () => {
  const error = refused(buildHttpRequest(httpTool("get", "/orders/{orderId}"), httpArguments()), "missing-path-parameter");
  assert.match(error.message, /\{orderId\}/);
});

test("an undeclared path parameter is refused, naming it", () => {
  const error = refused(buildHttpRequest(httpTool("get", "/orders"), httpArguments({ orderId: "A1" })), "undeclared-path-parameter");
  assert.match(error.message, /'orderId'/);
});

test("a path parameter given twice is refused", () => {
  const twice = httpArguments([["orderId", "A1"], ["orderId", "A2"]]);
  const error = refused(buildHttpRequest(httpTool("get", "/orders/{orderId}"), twice), "duplicate-path-parameter");
  assert.match(error.message, /'orderId'/);
});

test("an empty path parameter is refused", () => {
  refused(buildHttpRequest(httpTool("get", "/orders/{orderId}"), httpArguments({ orderId: "" })), "empty-path-parameter");
});

test("a placeholder that appears twice is filled twice from one parameter", () => {
  const tool = httpTool("get", "/a/{x}/b/{x}");
  assert.equal(built(buildHttpRequest(tool, httpArguments({ x: "1" }))).url, "https://api.example.com/a/1/b/1");
});

test("a query parameter with an empty name is refused", () => {
  refused(buildHttpRequest(httpTool("get", "/search"), httpArguments({}, [["", "x"]])), "empty-query-name");
});

test("a body is sent as plain JSON, without $type at any depth", () => {
  const body = {
    $type: "NewOrder",
    sku: "X",
    quantity: 2,
    lines: [{ $type: "Line", note: { $type: "Note", text: "gift" } }, "plain", 3, null, true],
    shipping: { $type: "Shipping.express", days: 1 },
  };
  const request = built(buildHttpRequest(httpTool("post", "/orders"), httpArguments({}, [], body)));
  assert.equal(request.method, "POST");
  assert.deepEqual(request.headers, { "content-type": "application/json" });
  assert.deepEqual(JSON.parse(request.body!), {
    sku: "X",
    quantity: 2,
    lines: [{ note: { text: "gift" } }, "plain", 3, null, true],
    shipping: { days: 1 },
  });
  assert.equal(request.body!.includes("$type"), false);
  // The value handed in is not changed.
  assert.equal(body.$type, "NewOrder");
});

test("the spec's body example", () => {
  const request = built(
    buildHttpRequest(httpTool("post", "/orders"), httpArguments({}, [], { $type: "NewOrder", sku: "X", quantity: 2 })),
  );
  assert.equal(request.body, '{"sku":"X","quantity":2}');
  assert.deepEqual(Object.entries(request.headers), [["content-type", "application/json"]]);
});

test("a body may be any one JSON value", () => {
  for (const [body, text] of [["A1", '"A1"'], [7, "7"], [true, "true"], [[1, { $type: "T", a: 1 }], '[1,{"a":1}]']] as const) {
    const request = built(buildHttpRequest(httpTool("put", "/orders/1"), httpArguments({}, [], body)));
    assert.equal(request.body, text);
    assert.deepEqual(request.headers, { "content-type": "application/json" });
  }
});

test("put, post and patch carry a body; get and delete refuse one", () => {
  for (const method of ["post", "put", "patch"] as const) {
    assert.equal(built(buildHttpRequest(httpTool(method, "/orders"), httpArguments({}, [], { a: 1 }))).method, method.toUpperCase());
  }
  for (const method of ["get", "delete"] as const) {
    const error = refused(buildHttpRequest(httpTool(method, "/orders"), httpArguments({}, [], { a: 1 })), "body-not-allowed");
    assert.match(error.message, new RegExp(method.toUpperCase()));
  }
});

test("a request with no body has no headers and no body key", () => {
  for (const method of ["get", "post", "delete"] as const) {
    const request = built(buildHttpRequest(httpTool(method, "/orders"), httpArguments()));
    assert.deepEqual(request.headers, {});
    assert.equal("body" in request, false);
  }
});

test("the package sets no idempotency or authorization header", () => {
  const request = built(buildHttpRequest(httpTool("post", "/orders"), httpArguments({}, [], { a: 1 })));
  assert.deepEqual(Object.keys(request.headers), ["content-type"]);
});

test("a host rebuilds the same request from stored configuration and structured arguments", () => {
  const tool = httpTool("post", "/orders/{orderId}/lines", "https://api.example.com/v1");
  const canonical = httpArguments({ orderId: "A 1" }, [["source", "agent"]], { $type: "Line", sku: "X" });
  const first = built(buildHttpRequest(tool, canonical));
  // What the request function is handed, sent to another process as JSON, with a stored copy of the tool.
  const structured = JSON.parse(
    JSON.stringify({
      pathParams: [{ name: "orderId", value: "A 1" }],
      query: [{ name: "source", value: "agent" }],
      body: { $type: "Line", sku: "X" },
    }),
  );
  const second = built(buildHttpRequest(JSON.parse(JSON.stringify(tool)), structured));
  assert.deepEqual(second, first);
  assert.equal(first.url, "https://api.example.com/v1/orders/A%201/lines?source=agent");
});

// ---- the parse-and-compare check ----------------------------------------------------------------
//
// With the rules above it in place, no input reaches this check with something to refuse, which is
// its purpose: it is what holds if one of them is ever wrong. So it is tested on its own, with the
// URLs those rules exist to prevent.

const baseV1 = { prefix: "https://api.example.com/v1", origin: "https://api.example.com", path: "/v1" };
const baseRoot = { prefix: "https://api.example.com", origin: "https://api.example.com", path: "" };

test("a URL a parser reads as it was built is answered", () => {
  const cases: readonly (readonly [string, string])[] = [
    ["/", ""],
    ["/orders/A%201%2F2", ""],
    ["/orders", "?q=a%26b&page=2"],
    ["/files/...", ""],
    ["/files/%252e%252e", ""],
    ["/a//b", "?empty="],
  ];
  for (const [path, query] of cases) {
    assert.deepEqual(underBaseUrl(baseV1, path, query), { ok: true, url: `https://api.example.com/v1${path}${query}` }, path + query);
  }
  assert.deepEqual(underBaseUrl(baseRoot, "/orders", ""), { ok: true, url: "https://api.example.com/orders" });
});

test("a URL whose text a parser would change is refused", () => {
  const cases: readonly (readonly [string, string])[] = [
    ["/files/..", ""],
    ["/files/../../admin", ""],
    ["/./orders", ""],
    ["/files/%2e%2e", ""],
    ["/files/.%2E/x", ""],
    ["/a b", ""],
    ["/a\\b", ""],
    ["/caf\u00e9", ""],
    ["/x\n", ""],
    ["/x\ty", ""],
    ["/x", "?q=a b"],
    ["/x", "?q='quoted'"],
  ];
  for (const [path, query] of cases) {
    assert.deepEqual(underBaseUrl(baseV1, path, query), { ok: false }, JSON.stringify(path + query));
  }
});

test("a character that ends the path early is refused, though the text is unchanged", () => {
  // Each of these parses to the very text it was given. What moves is where the path ends, which
  // would drop the fixed segment after the value: the server would see `/accounts/X`.
  for (const path of ["/accounts/X?/orders", "/accounts/X#/orders", "/accounts/X?admin=1", "/accounts/X#"]) {
    assert.equal(new URL(`https://api.example.com/v1${path}`).href, `https://api.example.com/v1${path}`, path);
    assert.deepEqual(underBaseUrl(baseV1, path, ""), { ok: false }, path);
  }
  // And in a query: a `#` would cut it short, and what follows would never be sent.
  assert.deepEqual(underBaseUrl(baseV1, "/orders", "?q=a#b"), { ok: false });
  assert.deepEqual(underBaseUrl(baseV1, "/orders", "?"), { ok: false });
});

test("a URL of another origin is refused", () => {
  // The base's text and the origin it was checked to have always agree; these are what a mismatch
  // would look like, in each part of an origin.
  for (const prefix of [
    "https://evil.example/v1",
    "http://api.example.com/v1",
    "https://api.example.com:8443/v1",
    "https://api.example.com.evil.example/v1",
  ]) {
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, "/orders", ""), { ok: false }, prefix);
  }
});

test("a URL with user information is refused, though its origin is the base URL's", () => {
  for (const prefix of ["https://user@api.example.com/v1", "https://user:secret@api.example.com/v1", "https://:secret@api.example.com/v1"]) {
    assert.equal(new URL(`${prefix}/orders`).origin, baseV1.origin);
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, "/orders", ""), { ok: false }, prefix);
  }
});

test("a path outside the base path is refused, at a segment boundary", () => {
  // The text under `/v10`, `/v1x` or another directory, against a base path of `/v1`.
  for (const prefix of ["https://api.example.com/v10", "https://api.example.com/v1x", "https://api.example.com/v2", "https://api.example.com"]) {
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, "/orders", ""), { ok: false }, prefix);
  }
  // A path that does not begin with `/` would run on from the base path's last segment.
  assert.deepEqual(underBaseUrl(baseV1, "0/orders", ""), { ok: false });
  assert.deepEqual(underBaseUrl(baseV1, "", ""), { ok: false });
});

test("text a parser would spell differently is refused, though it names the same request", () => {
  // The origin and the path the parser reports are the expected ones; only the text differs.
  for (const prefix of ["https://API.example.com/v1", "https://api.example.com:443/v1", "HTTPS://api.example.com/v1"]) {
    const parsed = new URL(`${prefix}/orders`);
    assert.equal(parsed.origin, baseV1.origin, prefix);
    assert.equal(parsed.pathname, "/v1/orders", prefix);
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, "/orders", ""), { ok: false }, prefix);
  }
});

test("a fragment is refused, though the path the parser reports is the expected one", () => {
  // The text before the `#` happens to end as the path would, so the path compares equal while the
  // path that was meant is in the fragment and would never be sent.
  for (const [prefix, path] of [["https://api.example.com/v1/orders#", "/orders"], ["https://api.example.com/v1/#", "/"]] as const) {
    const parsed = new URL(prefix + path);
    assert.equal(parsed.href, prefix + path);
    assert.equal(parsed.pathname, baseV1.path + path);
    assert.notEqual(parsed.hash, "");
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, path, ""), { ok: false }, prefix);
  }
});

test("text that is not a URL is refused", () => {
  for (const prefix of ["", "https://", "/v1", "api.example.com/v1"]) {
    assert.deepEqual(underBaseUrl({ ...baseV1, prefix }, "/orders", ""), { ok: false }, JSON.stringify(prefix));
  }
});

// ---- buildHttpRequest: values that are not what it expects --------------------------------------

test("arguments of the wrong shape are refused", () => {
  const tool = httpTool("get", "/orders/{orderId}");
  for (const value of [null, "A1", 7, [], undefined]) {
    refused(buildHttpRequest(tool, value), "arguments");
  }
  refused(buildHttpRequest(tool, { pathParams: "orderId" }), "arguments");
  refused(buildHttpRequest(tool, { pathParams: [{ name: "orderId" }] }), "arguments");
  refused(buildHttpRequest(tool, { pathParams: [{ name: "orderId", value: 7 }] }), "arguments");
  refused(buildHttpRequest(tool, { pathParams: [["orderId", "A1"]] }), "arguments");
  refused(buildHttpRequest(tool, { pathParams: [{ name: "orderId", value: "A1" }], query: [null] }), "arguments");
});

test("a tool that is not a well-formed http tool is refused", () => {
  const good = httpTool("get", "/orders");
  refused(buildHttpRequest({ ...good, kind: "function" } as never, httpArguments()), "not-an-http-tool");
  // Something that is not a tool at all is refused the same way, and is never a TypeError.
  for (const notATool of [null, undefined, "lookup_order", 7, [good]]) {
    const built = buildHttpRequest(notATool as never, httpArguments());
    assert.equal(built.ok, false);
    const { error } = built as { error: { code: string; tool: string; rule: string; message: string } };
    assert.deepEqual([error.code, error.tool, error.rule], ["invalid-request", "", "not-an-http-tool"]);
    assert.match(error.message, /^The tool: it is .+, not an http tool\.$/);
  }
  // A record that is not a tool says what it lacks, under its name when it has one that is a string.
  const notTools: readonly (readonly [unknown, string, string])[] = [
    [{}, "", "The tool: its kind is undefined, not 'http'."],
    [{ name: 7, kind: "http" }, "", "The tool: its 'config' is undefined, not a record."],
    [{ name: "lookup_order", kind: "http", config: null }, "lookup_order", "Tool 'lookup_order': its 'config' is null, not a record."],
    [{ name: "lookup_order", kind: "http", config: {} }, "lookup_order", "Tool 'lookup_order': its 'config.connection' is undefined, not a record."],
    [{ ...good, kind: "function" }, "lookup_order", `Tool 'lookup_order': its kind is the string "function", not 'http'.`],
  ];
  for (const [notATool, tool, message] of notTools) {
    const built = buildHttpRequest(notATool as never, httpArguments());
    assert.equal(built.ok, false);
    const { error } = built as { error: { tool: string; rule: string; message: string } };
    assert.deepEqual([error.tool, error.rule, error.message], [tool, "not-an-http-tool", message]);
  }
  refused(buildHttpRequest({ ...good, config: { ...good.config, method: "GET" } } as never, httpArguments()), "method");
  refused(buildHttpRequest({ ...good, config: { ...good.config, method: "head" } } as never, httpArguments()), "method");
  refused(buildHttpRequest(httpTool("get", "/orders", "https://api.example.com?x=1"), httpArguments()), "base-url");
  refused(buildHttpRequest(httpTool("get", "/orders", "ftp://api.example.com"), httpArguments()), "base-url");
  refused(buildHttpRequest(httpTool("get", "/orders", "https://API.example.com"), httpArguments()), "base-url");
  refused(buildHttpRequest(httpTool("get", "orders"), httpArguments()), "path-template");
  refused(buildHttpRequest(httpTool("get", "/orders/../admin"), httpArguments()), "path-template");
  refused(buildHttpRequest(httpTool("get", "/orders?x={id}", undefined, ["id"]), httpArguments({ id: "1" })), "path-template");
});

test("a stored http base URL is built against, since the scheme was settled when it was stored", () => {
  const tool = httpTool("get", "/orders/{id}", "http://localhost:8787/api");
  assert.equal(built(buildHttpRequest(tool, httpArguments({ id: "1" }))).url, "http://localhost:8787/api/orders/1");
});

test("no value can change where the request goes", () => {
  const tool = httpTool("get", "/orders/{id}", "https://api.example.com/v1");
  const attacks = [
    "https://evil.example/",
    "//evil.example/x",
    "\\\\evil.example\\x",
    "@evil.example",
    "x@evil.example",
    ":8080",
    "a/../../admin",
    "a?admin=1",
    "a#frag",
    "a\r\nHost: evil.example",
    "a\u0000b",
    "%2e%2e%2fadmin",
    "..%2f..%2fadmin",
    "‮",
    "a b",
    "\ud800",
  ];
  for (const value of attacks) {
    const result = buildHttpRequest(tool, httpArguments({ id: value }, [[value, value]]));
    const request = built(result);
    const url = new URL(request.url);
    assert.equal(url.href, request.url, JSON.stringify(value));
    assert.equal(url.origin, "https://api.example.com", JSON.stringify(value));
    const segments = url.pathname.split("/");
    assert.deepEqual(segments.slice(0, 3), ["", "v1", "orders"], JSON.stringify(value));
    assert.equal(segments.length, 4, JSON.stringify(value));
    assert.equal(url.hash, "");
    assert.equal(request.method, "GET");
    assert.equal([...url.searchParams].length, 1);
  }
});
