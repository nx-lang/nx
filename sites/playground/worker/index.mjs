/**
 * The playground's Cloudflare Worker. Its static assets are `dist/`, and this script runs only for
 * a request no file answered.
 *
 * The client router owns the prefix itself and one path segment below it (`/playground/<id>`), so
 * those get the shell. Everything else is not found: a missing asset answered with HTML turns a
 * broken path into a puzzling runtime error, and the playground serves no API.
 */
import { BASE_PATH } from "../base.mjs";

const SHELL = `${BASE_PATH}/index.html`;

/**
 * Whether the client router owns `pathname`: the prefix, with or without its slash, or one segment
 * below it that doesn't look like a file name.
 */
export function isShellPath(pathname) {
  if (pathname === BASE_PATH || pathname === `${BASE_PATH}/`) {
    return true;
  }
  if (!pathname.startsWith(`${BASE_PATH}/`)) {
    return false;
  }
  const rest = pathname.slice(BASE_PATH.length + 1).replace(/\/$/, "");
  return rest !== "" && !rest.includes("/") && !rest.includes(".");
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (isShellPath(url.pathname)) {
      // Fetched by its own path, so the shell carries the headers `_headers` gives it wherever it
      // is served, and a conditional request is answered 304 as usual.
      return env.ASSETS.fetch(new Request(new URL(SHELL, url), request));
    }
    return new Response("Not found\n", {
      status: 404,
      headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-cache" },
    });
  },
};
