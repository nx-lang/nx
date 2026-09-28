import react from "@vitejs/plugin-react";
import { copyFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { type Plugin, defineConfig } from "vite";
import { BASE_HREF, BASE_PATH } from "./base.mjs";

/**
 * Where the build goes. Files land under `dist/play/`, so the path of each file under `dist/` is
 * the path it is served at, and `dist/` is the directory the Cloudflare Worker serves.
 */
const outDir = fileURLToPath(new URL("./dist/play", import.meta.url));

/**
 * Copies the cache policy into the root of the served directory, where Cloudflare's static assets
 * read `_headers` from. It is not in `public/`, which would put it under the prefix and serve it as a
 * file.
 */
function cacheHeaders(): Plugin {
  return {
    name: "cache-headers",
    apply: "build",
    closeBundle() {
      copyFileSync(
        fileURLToPath(new URL("./_headers", import.meta.url)),
        fileURLToPath(new URL("./dist/_headers", import.meta.url)),
      );
    },
  };
}

/**
 * Serves the shell at the prefix without its slash, `/play`, in development as the Worker does in
 * production. Vite answers only `/play/`, and a shared link is `/play#code=…`.
 */
function prefixWithoutSlash(): Plugin {
  return {
    name: "prefix-without-slash",
    configureServer(server) {
      server.middlewares.use((request, _response, next) => {
        if (request.url === BASE_PATH || request.url?.startsWith(`${BASE_PATH}?`)) {
          request.url = BASE_HREF + request.url.slice(BASE_PATH.length);
        }
        next();
      });
    },
  };
}

/**
 * Where the website's dev server runs (`pnpm run dev` in `sites/website`), so the dev server here
 * can stand in for the whole domain. Set `NX_WEBSITE_DEV_URL` when it runs elsewhere.
 */
const websiteDevUrl = process.env["NX_WEBSITE_DEV_URL"] ?? "http://localhost:4321";

/**
 * Every path outside the prefix, the way the domain splits them in production: `/play` is the
 * playground's and everything else is the website's. Vite serves its own modules under the prefix
 * too (`/play/@vite/client`, `/play/src/...`), so nothing of the playground's is caught here.
 */
const outsidePrefix = `^/(?!${BASE_PATH.slice(1)}(?:[/?#]|$))`;

/**
 * `base` is the site's prefix: every asset URL Vite emits and `import.meta.env.BASE_URL` in the
 * client carry it, and the dev server serves the app at the same address production does, so a
 * path written against the origin's root breaks locally before it ships.
 *
 * There is no API proxy. The compiler and the language service are a WebAssembly module the client
 * loads into a worker, so `pnpm run dev` alone is the whole development setup.
 */
export default defineConfig({
  base: BASE_HREF,
  plugins: [react(), prefixWithoutSlash(), cacheHeaders()],
  build: { target: "es2022", outDir, emptyOutDir: true },
  worker: { format: "es" },
  server: {
    // The shared packages are workspace links into the repository, so dev needs to be allowed to
    // read above the app root.
    fs: { allow: [".", "../.."] },
    // The header's Docs link and each example's docs link are paths on the website. Forwarding them
    // to the website's dev server makes this one address behave like nxlang.org, links both ways.
    proxy: {
      [outsidePrefix]: {
        target: websiteDevUrl,
        changeOrigin: true,
        ws: true,
        configure: (proxy) => {
          // Where nothing listens, a connection can hang rather than be refused (WSL drops it), and
          // neither the proxy's `proxyTimeout` nor a request's `setTimeout` covers a connection that
          // never opens. So a plain timer fails the forwarded request, with an error the handler
          // below answers, if its connection has not opened within a few seconds. Once it has, the
          // website takes as long as it takes, a first compile included.
          proxy.on("proxyReq", (proxyRequest) => {
            const timer = setTimeout(() => {
              proxyRequest.destroy(new Error(`Could not connect to ${websiteDevUrl}`));
            }, 5_000);
            proxyRequest.once("socket", (socket) => {
              if (socket.connecting) {
                socket.once("connect", () => clearTimeout(timer));
              } else {
                clearTimeout(timer);
              }
            });
            proxyRequest.once("close", () => clearTimeout(timer));
          });
          proxy.on("error", (_error, _request, response) => {
            // A websocket upgrade hands over a socket rather than a response; nothing to answer.
            if (!("writeHead" in response) || response.headersSent) {
              return;
            }
            response.writeHead(502, { "content-type": "text/plain; charset=utf-8" });
            response.end(
              `This path belongs to the website, and its dev server isn't answering at ${websiteDevUrl}.\n` +
                "Start it with `pnpm run dev` in sites/website, or set NX_WEBSITE_DEV_URL.\n",
            );
          });
        },
      },
    },
  },
});
