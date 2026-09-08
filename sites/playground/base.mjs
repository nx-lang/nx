/**
 * The one place the site's path prefix is decided.
 *
 * Everything the playground serves lives under `/playground` — the gallery, each example's editor
 * view, the API and every asset — so that the rest of nxlang.org can later be served by something
 * else without this site changing. Three sides have to agree on the prefix: the server that routes
 * by it, the Vite config that builds asset URLs and proxies the API under it, and the client. The
 * client reads it back through `import.meta.env.BASE_URL`, which Vite fills from `base`, so the
 * bundle never imports this file.
 */

/** Where the site lives, with no trailing slash: the gallery's own address. */
export const BASE_PATH = "/playground";

/** The shell's `<base href>` and Vite's `base`: the prefix as a directory. */
export const BASE_HREF = `${BASE_PATH}/`;

/** Where the server's own routes — compile, language, health — are mounted. */
export const API_PREFIX = `${BASE_PATH}/api`;

/** What the hosting platform polls to decide the process is still able to serve. */
export const HEALTH_PATH = `${API_PREFIX}/health`;
