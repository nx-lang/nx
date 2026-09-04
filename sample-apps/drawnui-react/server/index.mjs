/**
 * Serves the built SPA and answers one compile request.
 *
 * The server exists only because there is no WASM build of the compiler yet. It holds no state and
 * has one route beyond static files, so replacing it later with an in-browser compiler removes this
 * file and changes nothing else.
 */
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { MAX_SOURCE_BYTES, compile } from "./compile.mjs";
import { COMPILE_PORT } from "./port.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const distRoot = join(appRoot, "dist");

/** Compile requests are bounded by the same limit the compiler applies to source. */
const MAX_BODY_BYTES = MAX_SOURCE_BYTES + 4096;

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

function sendJson(response, status, body) {
  const payload = JSON.stringify(body);
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(payload),
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

function serveStatic(request, response) {
  let requested;
  try {
    const url = new URL(request.url, "http://localhost");
    requested = normalize(decodeURIComponent(url.pathname)).replace(/^(\.\.[/\\])+/, "");
  } catch {
    // A percent-escape the decoder rejects, such as `/%ZZ`, names no file and never will. It is
    // the client's mistake to fix, and answering it must not be the last thing this process does.
    sendJson(response, 400, { error: "malformed path" });
    return;
  }
  if (!existsSync(distRoot)) {
    sendJson(response, 503, { error: "the SPA is not built; run `npm run build`" });
    return;
  }
  let file = join(distRoot, requested);
  if (!file.startsWith(distRoot) || !existsSync(file) || statSync(file).isDirectory()) {
    // Addresses the client router owns resolve to the shell. Anything that names a file does not:
    // answering a missing asset with HTML turns a broken path into a puzzling runtime error.
    if (extname(requested) !== "") {
      sendJson(response, 404, { error: `no such file: ${requested}` });
      return;
    }
    file = join(distRoot, "index.html");
  }
  response.writeHead(200, { "content-type": MIME[extname(file)] ?? "application/octet-stream" });
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
    if (request.url?.split("?")[0] === "/api/compile") {
      handleCompile(request, response).catch((error) => failRequest(response, error));
      return;
    }
    serveStatic(request, response);
  } catch (error) {
    failRequest(response, error);
  }
});

server.listen(COMPILE_PORT, () => {
  console.log(`drawnui fiddle listening on http://localhost:${COMPILE_PORT}`);
});
