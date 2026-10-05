import { isHttpToolMethod } from "./annotations.js";
import { isHttpTool, type NormalizedTool } from "./definition.js";
import { describeValue, isJsonObject, type JsonValue } from "./json.js";

/** One named value of a request, not encoded. */
export interface HttpParamData {
  readonly name: string;
  readonly value: string;
}

/**
 * The variable parts of one HTTP call, as an arguments function returned them: the structured
 * form of an `HttpArguments` record. `body` is as evaluated and keeps its `$type` keys.
 */
export interface HttpArgumentsData {
  readonly pathParams: readonly HttpParamData[];
  readonly query: readonly HttpParamData[];
  readonly body?: JsonValue;
}

/** A request to make. The package builds it and never sends it. */
export interface HttpRequestDescription {
  /** The tool's method, upper-cased. */
  readonly method: string;
  readonly url: string;
  /** `content-type: application/json` when there is a body, and nothing else. */
  readonly headers: Readonly<Record<string, string>>;
  /** The body as JSON text, with every `$type` key removed. */
  readonly body?: string;
}

/** Why a request could not be built: the tool, the rule it broke, and a sentence for a person. */
export interface HttpRequestError {
  readonly code: "invalid-request";
  readonly tool: string;
  readonly rule: HttpRequestRule;
  readonly message: string;
}

export type HttpRequestRule =
  | "not-an-http-tool"
  | "method"
  | "base-url"
  | "path-template"
  | "arguments"
  | "missing-path-parameter"
  | "undeclared-path-parameter"
  | "duplicate-path-parameter"
  | "empty-path-parameter"
  | "dot-segment"
  | "empty-query-name"
  | "body-not-allowed"
  | "body-not-json"
  | "outside-base-url";

export type BuildHttpRequestResult =
  | { readonly ok: true; readonly request: HttpRequestDescription }
  | { readonly ok: false; readonly error: HttpRequestError };

/** What `buildHttpCall` answers: the request, and the arguments it was built from, as they were read. */
export type BuildHttpCallResult =
  | { readonly ok: true; readonly request: HttpRequestDescription; readonly arguments: HttpArgumentsData }
  | { readonly ok: false; readonly error: HttpRequestError };

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;

/**
 * The characters a path template may hold literally: RFC 3986's `pchar` without the percent sign,
 * and `/`. None of them is rewritten by a URL parser, and none ends the path.
 */
const PATH_LITERAL = /^[A-Za-z0-9\-._~!$&'()*+,;=:@/]$/;

const HEX = /^[0-9A-Fa-f]$/;

/**
 * Whether a URL parser would take `segment` for `.` or `..`. It reads `%2e` as a dot in either
 * case, so `.%2E` walks up a directory exactly as `..` does.
 */
function isDotSegment(segment: string): boolean {
  const plain = segment.replace(/%2e/gi, ".");
  return plain === "." || plain === "..";
}

/** One piece of a path template: text to copy, or a placeholder to fill. */
type PathPart = { readonly literal: string } | { readonly placeholder: string };

export type HttpPathTemplate =
  | { readonly ok: true; readonly parts: readonly PathPart[]; readonly placeholders: readonly string[] }
  | { readonly ok: false; readonly problem: string };

/**
 * Reads an `HttpTool`'s path template: it begins with `/`, holds URL path characters and `{name}`
 * placeholders whose names are identifiers, has balanced braces, and no literal segment is `.` or
 * `..`. Answers the placeholder names, each once, in the order they first appear.
 */
export function parseHttpPath(path: string): HttpPathTemplate {
  if (!path.startsWith("/")) {
    return { ok: false, problem: "it does not begin with '/'" };
  }
  const parts: PathPart[] = [];
  const placeholders: string[] = [];
  let literal = "";
  for (let index = 0; index < path.length; index += 1) {
    const character = path[index]!;
    if (character === "{") {
      const close = path.indexOf("}", index);
      const name = close < 0 ? "" : path.slice(index + 1, close);
      if (close < 0 || name.includes("{")) {
        return { ok: false, problem: `the '{' at offset ${index} is not closed` };
      }
      if (!IDENTIFIER.test(name)) {
        return { ok: false, problem: `the placeholder '{${name}}' is not named by an identifier` };
      }
      if (literal !== "") {
        parts.push({ literal });
        literal = "";
      }
      parts.push({ placeholder: name });
      if (!placeholders.includes(name)) {
        placeholders.push(name);
      }
      index = close;
    } else if (character === "}") {
      return { ok: false, problem: `the '}' at offset ${index} closes no placeholder` };
    } else if (character === "%") {
      if (!HEX.test(path[index + 1] ?? "") || !HEX.test(path[index + 2] ?? "")) {
        return { ok: false, problem: `the '%' at offset ${index} is not followed by two hexadecimal digits` };
      }
      literal += path.slice(index, index + 3);
      index += 2;
    } else if (PATH_LITERAL.test(character)) {
      literal += character;
    } else {
      return { ok: false, problem: `the character ${JSON.stringify(character)} at offset ${index} is not a URL path character` };
    }
  }
  if (literal !== "") {
    parts.push({ literal });
  }
  // A literal segment is one no placeholder contributes to. One a placeholder does contribute to is
  // checked when it is filled, since only then is it known.
  const template = parts.map((part) => ("literal" in part ? part.literal : "{}")).join("");
  for (const segment of template.split("/")) {
    if (!segment.includes("{}") && isDotSegment(segment)) {
      return { ok: false, problem: `it has the segment '${segment}'` };
    }
  }
  return { ok: true, parts, placeholders };
}

export type HttpBaseUrl =
  | {
      readonly ok: true;
      /** The base URL without a trailing `/`: what a path, which begins with one, is appended to. */
      readonly prefix: string;
      readonly origin: string;
      /** The base URL's path without a trailing `/`: empty when it is the origin alone. */
      readonly path: string;
    }
  | { readonly ok: false; readonly problem: string };

/**
 * Reads a connection's `baseUrl`: an absolute `https` URL (`http` too when `allowInsecure` is set)
 * with no query, fragment or user information, written the way a URL parser writes it.
 *
 * <para>The last condition is what lets a request be checked after it is built. A request's URL is
 * the base URL's text with a path appended, and it is refused when parsing would change it. A base
 * URL the parser itself would rewrite, by lower-casing a host, dropping a default port or resolving
 * a dot segment, would make every such check fail, so it is refused here, with the spelling to
 * write instead.</para>
 */
export function parseHttpBaseUrl(baseUrl: string, options: { readonly allowInsecure?: boolean } = {}): HttpBaseUrl {
  let url: URL;
  try {
    url = new URL(baseUrl);
  } catch {
    return { ok: false, problem: "it is not an absolute URL" };
  }
  const scheme = url.protocol.slice(0, -1);
  if (scheme !== "https" && !(scheme === "http" && options.allowInsecure === true)) {
    return { ok: false, problem: `its scheme is '${scheme}', not 'https'` };
  }
  if (baseUrl.includes("?")) {
    return { ok: false, problem: "it has a query" };
  }
  if (baseUrl.includes("#")) {
    return { ok: false, problem: "it has a fragment" };
  }
  if (url.username !== "" || url.password !== "" || baseUrl.slice(url.protocol.length + 2).split("/")[0]!.includes("@")) {
    return { ok: false, problem: "it has user information" };
  }
  const prefix = baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
  const canonical = url.href.endsWith("/") ? url.href.slice(0, -1) : url.href;
  if (canonical !== prefix) {
    return { ok: false, problem: `a URL parser reads it as '${canonical}'; write it that way` };
  }
  const path = url.pathname.endsWith("/") ? url.pathname.slice(0, -1) : url.pathname;
  return { ok: true, prefix, origin: url.origin, path };
}

/**
 * The URL of `path` and `query` under `base`, when a URL parser reads it exactly as it was put
 * together, and nothing otherwise.
 *
 * <para>The text is the base URL, then `path`, then `query`. It is answered only when it parses,
 * parsing leaves the text unchanged, and the parser finds in it what was put there: the base URL's
 * origin, no user information, a path that is the base URL's path followed by `path`, a query that
 * is `query` and no fragment. A `path` that begins with `/` therefore lies under the base path at
 * a segment boundary, so nothing under `/v1` is mistaken for `/v10`.</para>
 *
 * <para>This is the structural form of "the request stays under `baseUrl`". Comparing the parts
 * and not only the text is what catches a character that should have been encoded and was not: a
 * `?` or a `#` in a path leaves the text as it was and moves where the path ends, which would drop
 * the fixed segments after it.</para>
 */
export function underBaseUrl(
  base: { readonly prefix: string; readonly origin: string; readonly path: string },
  path: string,
  query: string,
): { readonly ok: true; readonly url: string } | { readonly ok: false } {
  const url = base.prefix + path + query;
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return { ok: false };
  }
  const isAsBuilt =
    path.startsWith("/") &&
    parsed.href === url &&
    parsed.origin === base.origin &&
    parsed.username === "" &&
    parsed.password === "" &&
    parsed.pathname === base.path + path &&
    parsed.search === query &&
    parsed.hash === "";
  return isAsBuilt ? { ok: true, url } : { ok: false };
}

const LONE_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/g;

/** Percent-encodes every UTF-8 byte of `value` outside `A-Z a-z 0-9 - . _ ~`. */
function encodeUnreserved(value: string): string {
  // `encodeURIComponent` leaves `! ' ( ) *` as well, and throws on a lone surrogate, which has no
  // UTF-8 form; one is first turned into U+FFFD, as a `TextEncoder` would turn it.
  return encodeURIComponent(value.replace(LONE_SURROGATE, "\uFFFD")).replace(
    /[!'()*]/g,
    (character) => `%${character.charCodeAt(0).toString(16).toUpperCase()}`,
  );
}

/** `value` without its `$type` keys, at every depth. */
function withoutTypeKeys(value: JsonValue): JsonValue {
  if (Array.isArray(value)) {
    return (value as readonly JsonValue[]).map(withoutTypeKeys);
  }
  if (isJsonObject(value)) {
    const copy: Record<string, JsonValue> = {};
    for (const [key, held] of Object.entries(value)) {
      if (key !== "$type") {
        copy[key] = withoutTypeKeys(held);
      }
    }
    return copy;
  }
  return value;
}

/** The `{ name, value }` list under `key` of an `HttpArguments` value, or what is wrong with it. */
function paramList(value: { readonly [key: string]: unknown }, key: string): readonly HttpParamData[] | string {
  const held = value[key];
  if (held === undefined) {
    return [];
  }
  if (!Array.isArray(held)) {
    return `'${key}' is not a list`;
  }
  const params: HttpParamData[] = [];
  for (const [index, item] of held.entries()) {
    if (!isJsonObject(item) || typeof item["name"] !== "string" || typeof item["value"] !== "string") {
      return `'${key}[${index}]' is not a record with a string 'name' and a string 'value'`;
    }
    params.push({ name: item["name"], value: item["value"] });
  }
  return params;
}

/**
 * The structured form of an `HttpArguments` value: its path and query parameters as
 * `{ name, value }` lists and its body as evaluated. Accepts the canonical record an arguments
 * function returns and the structured form itself. Answers a sentence when the value is neither.
 */
export function readHttpArguments(value: unknown): HttpArgumentsData | string {
  if (!isJsonObject(value)) {
    return "the arguments are not a record";
  }
  const pathParams = paramList(value, "pathParams");
  if (typeof pathParams === "string") {
    return pathParams;
  }
  const query = paramList(value, "query");
  if (typeof query === "string") {
    return query;
  }
  const body = value["body"];
  return body === undefined ? { pathParams, query } : { pathParams, query, body };
}

/**
 * Builds the request an `http` tool makes for one evaluated `HttpArguments` value. Pure: it reads
 * nothing but its arguments and sends nothing.
 *
 * <para>The method, the scheme, the host, the port and the fixed part of the path come from the
 * tool, and nothing in the arguments can change them. A value fills a placeholder or a query
 * parameter with every byte outside `A-Z a-z 0-9 - . _ ~` percent-encoded, so it cannot hold a
 * `/`, `?`, `#`, `&` or `=`; a path segment that comes out as `.` or `..` is refused, since those
 * are the two a parser would act on; and the finished URL is parsed and refused unless parsing
 * leaves its text unchanged, its origin is the base URL's and its path is under the base URL's
 * path.</para>
 *
 * <para>A server that receives a tool name and structured arguments from another process calls
 * this with its own copy of the tool definition and gets the request that process would have
 * built.</para>
 */
export function buildHttpRequest(tool: NormalizedTool, httpArguments: unknown): BuildHttpRequestResult {
  const built = buildHttpCall(tool, httpArguments);
  return built.ok ? { ok: true, request: built.request } : built;
}

/**
 * The builder behind {@link buildHttpRequest}, which also answers the arguments in the structured
 * form it read them in: what `execute` hands a request function beside the request. One read
 * serves both, so the two cannot disagree.
 */
export function buildHttpCall(tool: NormalizedTool, httpArguments: unknown): BuildHttpCallResult {
  // A caller that holds a copy of a definition holds parsed JSON, whatever its type says, so
  // nothing of the tool is read before it is looked at. A tool with no name is named by none.
  const given: unknown = tool;
  const name = isJsonObject(given) && typeof given["name"] === "string" ? given["name"] : "";
  const fail = (rule: HttpRequestRule, message: string): BuildHttpCallResult => ({
    ok: false,
    error: { code: "invalid-request", tool: name, rule, message: `${name === "" ? "The tool" : `Tool '${name}'`}: ${message}` },
  });

  if (!isJsonObject(given)) {
    return fail("not-an-http-tool", `it is ${describeValue(given)}, not an http tool.`);
  }
  if (!isHttpTool(tool)) {
    return fail("not-an-http-tool", `its kind is ${describeValue(given["kind"])}, not 'http'.`);
  }
  if (!isJsonObject(tool.config) || !isJsonObject(tool.config.connection)) {
    const [member, held]: [string, unknown] = isJsonObject(tool.config) ? ["config.connection", tool.config.connection] : ["config", tool.config];
    return fail("not-an-http-tool", `its '${member}' is ${describeValue(held)}, not a record.`);
  }
  const { method, path: pathTemplate } = tool.config;
  if (!isHttpToolMethod(method)) {
    return fail("method", `its method, ${JSON.stringify(method)}, is not one of get, post, put, patch and delete.`);
  }
  // Both schemes are read here: which one a deployment allows was settled when the definition was
  // made, and the origin check below holds the request to whichever the connection has.
  const base = parseHttpBaseUrl(String(tool.config.connection.baseUrl), { allowInsecure: true });
  if (!base.ok) {
    return fail("base-url", `the base URL of connection '${tool.config.connection.name}' cannot be used: ${base.problem}.`);
  }
  const template = parseHttpPath(String(pathTemplate));
  if (!template.ok) {
    return fail("path-template", `the path '${pathTemplate}' cannot be used: ${template.problem}.`);
  }
  const data = readHttpArguments(httpArguments);
  if (typeof data === "string") {
    return fail("arguments", `${data}.`);
  }

  const values = new Map<string, string>();
  for (const { name, value } of data.pathParams) {
    if (!template.placeholders.includes(name)) {
      return fail("undeclared-path-parameter", `the path '${pathTemplate}' has no placeholder for the path parameter '${name}'.`);
    }
    if (values.has(name)) {
      return fail("duplicate-path-parameter", `the path parameter '${name}' is given twice.`);
    }
    if (value === "") {
      return fail("empty-path-parameter", `the path parameter '${name}' is empty.`);
    }
    values.set(name, value);
  }
  let path = "";
  for (const part of template.parts) {
    if ("literal" in part) {
      path += part.literal;
      continue;
    }
    const value = values.get(part.placeholder);
    if (value === undefined) {
      return fail("missing-path-parameter", `the arguments hold no path parameter for the placeholder '{${part.placeholder}}'.`);
    }
    path += encodeUnreserved(value);
  }
  for (const segment of path.split("/")) {
    if (isDotSegment(segment)) {
      return fail("dot-segment", `the path has the segment '${segment}' once its placeholders are filled.`);
    }
  }

  const pairs: string[] = [];
  for (const { name, value } of data.query) {
    if (name === "") {
      return fail("empty-query-name", "a query parameter has an empty name.");
    }
    pairs.push(`${encodeUnreserved(name)}=${encodeUnreserved(value)}`);
  }
  // The rules above should make this impossible to fail, and it is what says so. The request's URL
  // is the one this answers, so the check cannot be dropped and a URL still come out.
  const checked = underBaseUrl(base, path, pairs.length === 0 ? "" : `?${pairs.join("&")}`);
  if (!checked.ok) {
    return fail("outside-base-url", `the URL it builds is not under the base URL of connection '${tool.config.connection.name}'.`);
  }
  const { url } = checked;

  const request = { method: method.toUpperCase(), url, headers: {} };
  if (data.body === undefined) {
    return { ok: true, request, arguments: data };
  }
  if (method === "get" || method === "delete") {
    return fail("body-not-allowed", `the arguments hold a body, and a ${method.toUpperCase()} request carries none.`);
  }
  let body: string | undefined;
  try {
    body = JSON.stringify(withoutTypeKeys(data.body));
  } catch {
    body = undefined;
  }
  if (body === undefined) {
    return fail("body-not-json", "the body is not a JSON value.");
  }
  return { ok: true, request: { ...request, headers: { "content-type": "application/json" }, body }, arguments: data };
}
