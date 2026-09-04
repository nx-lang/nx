import react from "@vitejs/plugin-react";
import { type Plugin, defineConfig } from "vite";
import type { ServerResponse } from "node:http";
import { connect } from "node:net";
import { COMPILE_PORT } from "./server/port.mjs";

/**
 * Where `npm start` serves `POST /api/compile`. The port comes from the compile server itself, so an
 * ambient `PORT` moves the server and this proxy together.
 */
const COMPILE_HOST = "127.0.0.1";
const COMPILE_SERVER = `http://${COMPILE_HOST}:${COMPILE_PORT}`;

/** How long the reachability probe waits before calling the compile server absent. */
const PROBE_MS = 1000;

/** A ceiling on a proxied compile, below the 8s the app gives up after. */
const PROXY_TIMEOUT_MS = 7000;

const START_IT = "start it with `npm start`, or run `npm run dev:all` to start both";

function sendError(response: ServerResponse, status: number, message: string) {
  if (response.headersSent || response.writableEnded) {
    response.end();
    return;
  }
  const payload = JSON.stringify({ error: message });
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(payload),
  });
  response.end(payload);
}

/** Whether anything is listening where the compile server should be. */
function isCompileServerUp(): Promise<boolean> {
  return new Promise((fulfil) => {
    const socket = connect({ host: COMPILE_HOST, port: COMPILE_PORT });
    const settle = (up: boolean) => {
      socket.destroy();
      fulfil(up);
    };
    socket.setTimeout(PROBE_MS);
    socket.once("connect", () => settle(true));
    socket.once("timeout", () => settle(false));
    socket.once("error", () => settle(false));
  });
}

/**
 * Answers `/api` itself when the compile server is not running.
 *
 * Without this the app reports the compile timing out, which reads like a slow compiler rather than
 * an absent one — and the proxy alone cannot say which, because a connection to an unused port does
 * not always come back refused. Under WSL it hangs instead, so the proxy's own error never fires and
 * the request stalls until the app gives up. A connect probe answers in a millisecond when the
 * server is up and names the real problem when it is not.
 */
function compileServerProbe(): Plugin {
  return {
    name: "compile-server-probe",
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if (!request.url?.startsWith("/api")) {
          next();
          return;
        }
        isCompileServerUp().then((up) => {
          if (up) {
            next();
            return;
          }
          sendError(
            response as ServerResponse,
            502,
            `the compile server is not running at ${COMPILE_SERVER} — ${START_IT}`,
          );
        }, next);
      });
    },
  };
}

/**
 * The vendored DrawnUI source is a Vite project: it imports the CanvasKit wasm binary with `?url`
 * and loads its fonts from `publicDir`. Both settings below mirror `samples/vite.shared.ts`
 * upstream so the vendored tree runs unmodified.
 */
export default defineConfig({
  plugins: [react(), compileServerProbe()],
  build: { target: "esnext" },
  server: {
    // The NX TextMate grammar is imported from the repository rather than copied, so dev needs to
    // be allowed to read above the app root.
    fs: { allow: [".", "../.."] },
    proxy: {
      "/api": {
        target: COMPILE_SERVER,
        // The probe clears the common case; this covers a server that dies mid-request, which would
        // otherwise hang the same way. Only the upstream request is bounded: bounding the incoming
        // one as well would cut the client off at the same deadline, so the 502 below — the whole
        // point of the timeout — would never reach it.
        proxyTimeout: PROXY_TIMEOUT_MS,
        configure(proxy) {
          proxy.on("error", (error, _request, response) => {
            // A failed websocket upgrade hands back a raw socket, which has no HTTP reply to write.
            if (!("writeHead" in response)) {
              response.destroy();
              return;
            }
            sendError(
              response,
              502,
              `the compile server at ${COMPILE_SERVER} failed mid-request (${error.message}) — ${START_IT}`,
            );
          });
        },
      },
    },
  },
});
