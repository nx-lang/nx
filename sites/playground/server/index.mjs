/**
 * Serves the built SPA under the site's prefix, answers compile requests, answers the source pane's
 * language queries, and reports its own health.
 *
 * The server exists only because there is no WASM build of the compiler or the language service
 * yet. It holds no state and has three routes beyond static files — compile, language and health,
 * all under `/playground/api/` — so replacing it later with in-browser analysis removes this file
 * and changes nothing else. Everything it serves lives under the prefix from `base.mjs`: the root
 * redirects there, and any other address outside it is not found, so a misrouted request shows up
 * as an obvious error rather than as a second copy of the site.
 */
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, normalize, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { API_PREFIX, BASE_PATH, HEALTH_PATH } from "../base.mjs";
import { MAX_SOURCE_BYTES, compile } from "./compile.mjs";
import { LANGUAGE_ROUTE, languageListener } from "./language.mjs";
import { COMPILE_PORT } from "./port.mjs";
import { startWatchdog } from "./watchdog.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
/** Where the built SPA is. Tests point this at a stand-in so they need not run a build first. */
const distRoot = resolve(process.env.PLAYGROUND_DIST ?? join(appRoot, "dist"));

const COMPILE_PATH = `${API_PREFIX}/compile`;

/** Compile requests are bounded by the same limit the compiler applies to source. */
const MAX_BODY_BYTES = MAX_SOURCE_BYTES + 4096;

/**
 * What an edge cache may keep, decided here so it holds whichever edge sits in front.
 *
 * Vite names everything under `assets/` by content hash, so a new build changes the names and the
 * old ones can be held forever. The shell is what names them, so it is fetched fresh every time.
 * Fonts and images keep their names across builds and are held for a day at most. Nothing the API
 * says is worth keeping: every answer depends on the body that asked for it.
 */
const CACHE = {
  hashed: "public, max-age=31536000, immutable",
  static: "public, max-age=86400",
  shell: "no-cache",
  api: "no-store",
};

const MIME = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".jpg": "image/jpeg",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".ttf": "font/ttf",
  ".wasm": "application/wasm",
};

/** Every JSON answer is an API answer or an error; neither is worth caching. */
function sendJson(response, status, body) {
  const payload = JSON.stringify(body);
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(payload),
    "cache-control": CACHE.api,
  });
  response.end(payload);
}

function readBody(request) {
  return new Promise((fulfil, reject) => {
    const chunks = [];
    let size = 0;
    request.on("data", (chunk) => {
      size += chunk.length;
      if (size > MAX_BODY_BYTES) {
        // Stop reading, but leave the socket alive long enough to answer: a client that is told
        // 413 can shrink its request, while a destroyed connection tells it nothing at all.
        request.pause();
        reject(Object.assign(new Error("request body too large"), { status: 413 }));
        return;
      }
      chunks.push(chunk);
    });
    request.on("end", () => fulfil(Buffer.concat(chunks).toString("utf8")));
    request.on("error", reject);
  });
}

async function handleCompile(request, response) {
  if (request.method !== "POST") {
    sendJson(response, 405, { error: "use POST" });
    return;
  }
  let source;
  try {
    const body = await readBody(request);
    const parsed = JSON.parse(body);
    source = parsed?.source;
  } catch (error) {
    response.on("finish", () => request.destroy());
    sendJson(response, error.status ?? 400, { error: error.message });
    return;
  }
  if (typeof source !== "string") {
    sendJson(response, 400, { error: "body must be { source: string }" });
    return;
  }

  try {
    sendJson(response, 200, compile(source));
  } catch (error) {
    // A compile that fails outside diagnostics is the app's problem, not the visitor's, but it must
    // not take the server down with it.
    const status = error instanceof RangeError ? 413 : 500;
    sendJson(response, status, { error: error.message });
  }
}

/**
 * Says the process can serve, and says nothing when it cannot.
 *
 * Answered inline on the request thread, which is the same thread compile and language calls hold
 * while they are inside the native binding. That is deliberate: a process stuck in one of those
 * calls cannot answer this either, so whoever is checking — the hosting platform before it switches
 * traffic to a new deployment, or a person — sees the truth. A health check answered from anywhere
 * else would report a stuck server as fine. Replacing a stuck process is the watchdog's job.
 */
function handleHealth(request, response) {
  // `HEAD` too: external monitors commonly probe with it, and Node drops the body on its own.
  if (request.method !== "GET" && request.method !== "HEAD") {
    sendJson(response, 405, { error: "use GET" });
    return;
  }
  sendJson(response, 200, { ok: true });
}

/** The root is the only address outside the prefix that answers with anything but not found. */
function redirectToSite(response) {
  // Temporary: the root will become a home page, and a permanent redirect would be remembered by
  // browsers past that day.
  response.writeHead(302, { location: BASE_PATH, "cache-control": CACHE.shell });
  response.end();
}

/**
 * Serves a file from `dist/`, or the shell for an address the client router owns.
 *
 * `path` is under the prefix; what follows the prefix names the file. Anything that names a file
 * and misses is answered not found: answering a missing asset with HTML turns a broken path into a
 * puzzling runtime error.
 */
function serveSite(response, path) {
  let requested;
  try {
    requested = normalize(decodeURIComponent(path.slice(BASE_PATH.length))).replace(/^([/\\]|\.\.[/\\])+/, "");
  } catch {
    // A percent-escape the decoder rejects, such as `/%ZZ`, names no file and never will. It is
    // the client's mistake to fix, and answering it must not be the last thing this process does.
    sendJson(response, 400, { error: "malformed path" });
    return;
  }
  if (!existsSync(distRoot)) {
    sendJson(response, 503, { error: "the site is not built; run `pnpm run build`" });
    return;
  }
  let file = join(distRoot, requested);
  let cache = CACHE.static;
  if (file !== distRoot && !file.startsWith(distRoot + sep)) {
    sendJson(response, 404, { error: `no such file: ${requested}` });
    return;
  }
  if (!existsSync(file) || statSync(file).isDirectory()) {
    if (extname(requested) !== "") {
      sendJson(response, 404, { error: `no such file: ${requested}` });
      return;
    }
    file = join(distRoot, "index.html");
  }
  // Decided by the file, not by how it was asked for: the shell is the shell whether it was reached
  // by a route or by its own name, and an intermediary that kept it for a day by name would keep
  // naming a build's assets after the next deploy replaced them.
  const inDist = relative(distRoot, file);
  if (inDist === "index.html") {
    cache = CACHE.shell;
  } else if (inDist.startsWith(`assets${sep}`)) {
    cache = CACHE.hashed;
  }
  response.writeHead(200, {
    "content-type": MIME[extname(file)] ?? "application/octet-stream",
    "cache-control": cache,
  });
  const stream = createReadStream(file);
  // A read that fails after the headers are out — the file removed mid-request, say — arrives as an
  // event rather than a throw, and an unhandled one on a stream ends the process.
  stream.on("error", (error) => {
    console.error(`failed to read ${requested}:`, error);
    response.end();
  });
  stream.pipe(response);
}

/**
 * Answers a request that failed outside any handler's own error handling.
 *
 * The service is a single process serving every visitor, so one request must never be able to end
 * it. Anything that reaches here is a fault in this file rather than something the client can fix.
 */
function failRequest(response, error) {
  console.error("request failed:", error);
  if (response.headersSent || response.writableEnded) {
    response.end();
    return;
  }
  sendJson(response, 500, { error: "request failed" });
}

const server = createServer((request, response) => {
  try {
    const path = request.url?.split("?")[0] ?? "";
    if (path === "/") {
      redirectToSite(response);
      return;
    }
    if (path !== BASE_PATH && !path.startsWith(`${BASE_PATH}/`)) {
      sendJson(response, 404, { error: `nothing is served outside ${BASE_PATH}` });
      return;
    }
    if (path === COMPILE_PATH) {
      handleCompile(request, response).catch((error) => failRequest(response, error));
      return;
    }
    if (path === HEALTH_PATH) {
      handleHealth(request, response);
      return;
    }
    if (path.startsWith(LANGUAGE_ROUTE)) {
      // The handler writes its own headers; this one is merged in underneath them.
      response.setHeader("cache-control", CACHE.api);
      languageListener(request, response);
      return;
    }
    if (path === API_PREFIX || path.startsWith(`${API_PREFIX}/`)) {
      // An API path with no handler is a wrong address, not a page the client router owns.
      sendJson(response, 404, { error: `no such route: ${path}` });
      return;
    }
    serveSite(response, path);
  } catch (error) {
    failRequest(response, error);
  }
});

startWatchdog();
server.listen(COMPILE_PORT, () => {
  console.log(`NX playground listening on http://localhost:${COMPILE_PORT}${BASE_PATH}`);
});
