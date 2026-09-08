import react from "@vitejs/plugin-react";
import { type Plugin, defineConfig } from "vite";
import type { ServerResponse } from "node:http";
import { connect } from "node:net";
import { API_PREFIX, BASE_HREF } from "./base.mjs";
import { COMPILE_PORT } from "./server/port.mjs";

/**
 * Where `pnpm start` serves `POST /playground/api/compile` and `POST /playground/api/language/*`.
 * The port comes from the compile server itself, so an ambient `PORT` moves the server and this
 * proxy together; the prefix comes from `base.mjs`, so the two cannot disagree on that either.
 */
const COMPILE_HOST = "127.0.0.1";
const COMPILE_SERVER = `http://${COMPILE_HOST}:${COMPILE_PORT}`;

/** How long the reachability probe waits before calling the compile server absent. */
const PROBE_MS = 1000;

/** A ceiling on a proxied compile, below the 8s the app gives up after. */
const PROXY_TIMEOUT_MS = 7000;

const START_IT = "start it with `pnpm start`, or run `pnpm run dev:all` to start both";

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
 * Answers the API prefix itself when the compile server is not running.
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
        if (!request.url?.startsWith(API_PREFIX)) {
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
 * Puts the site's `<base href>` into the shell from the same constant Vite's `base` comes from.
 *
 * Editor addresses are nested (`/playground/shapes`), and DrawnUI loads the images the examples
 * name by relative path; without a `<base>` they would resolve against the route. Injecting it
 * here rather than writing it in `index.html` keeps `base.mjs` the only place the prefix is spelled.
 */
function siteBase(): Plugin {
  return {
    name: "site-base",
    transformIndexHtml: () => [{ tag: "base", attrs: { href: BASE_HREF }, injectTo: "head-prepend" }],
  };
}

/**
 * The vendored DrawnUI source is a Vite project: it imports the CanvasKit wasm binary with `?url`
 * and loads its fonts from `publicDir`. The `build` and `fs` settings below mirror
 * `samples/vite.shared.ts` upstream so the vendored tree runs unmodified.
 *
 * `base` is the site's prefix: every asset URL Vite emits and `import.meta.env.BASE_URL` in the
 * client carry it, and the dev server serves the app at the same address production does, so a
 * path written against the origin's root breaks locally before it ships.
 */
export default defineConfig({
  base: BASE_HREF,
  plugins: [react(), siteBase(), compileServerProbe()],
  build: { target: "esnext" },
  server: {
    // The shared packages are workspace links into the repository, so dev needs to be allowed to
    // read above the app root.
    fs: { allow: [".", "../.."] },
    proxy: {
      // Every route the compile server answers — compile, language, health — shares the one
      // prefix, so one rule carries them together.
      [API_PREFIX]: {
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
