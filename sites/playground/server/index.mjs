/**
 * Serves the built SPA under the site's prefix and reports its own health.
 *
 * Compilation and language queries happen in the visitor's browser, in a worker over the
 * WebAssembly build of the compiler, so this process holds no state, does no work per visitor and
 * has one route beyond static files: health, which gates a new deployment. Everything it serves
 * lives under the prefix from `base.mjs`: the root redirects there, and any other address outside
 * it is not found, so a misrouted request shows up as an obvious error rather than as a second copy
 * of the site.
 */
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, normalize, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { API_PREFIX, BASE_PATH, HEALTH_PATH } from "../base.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
/** Where the built SPA is. Tests point this at a stand-in so they need not run a build first. */
const distRoot = resolve(process.env.PLAYGROUND_DIST ?? join(appRoot, "dist"));

/** The port the site listens on. Railway sets `PORT`; the Dockerfile pins it to 8080. */
const PORT = Number(process.env.PORT ?? 8080);

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

/** Every JSON answer is health or an error; neither is worth caching. */
function sendJson(response, status, body) {
  const payload = JSON.stringify(body);
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(payload),
    "cache-control": CACHE.api,
  });
  response.end(payload);
}

/**
 * Says the process can serve.
 *
 * This is what the hosting platform polls before it switches traffic to a new deployment. Nothing
 * a visitor does reaches this process's event loop any more, so there is no longer a way for it to
 * be alive and unable to answer.
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
    if (path === HEALTH_PATH) {
      handleHealth(request, response);
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

server.listen(PORT, () => {
  console.log(`NX playground listening on http://localhost:${PORT}${BASE_PATH}`);
});
