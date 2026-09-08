/**
 * Where the site lives, as the client sees it.
 *
 * Vite fills `import.meta.env.BASE_URL` from `base` in `vite.config.ts`, which reads `base.mjs`, so
 * this is the same `/playground/` the server routes by, without the bundle importing a server file.
 */
export const BASE_URL: string = import.meta.env.BASE_URL;

/** The gallery's address: the prefix with no trailing slash. */
export const SITE_ROOT = BASE_URL.replace(/\/$/, "");

/** Where the server's own routes — compile, language, health — are mounted. */
export const API_ROOT = `${SITE_ROOT}/api`;
