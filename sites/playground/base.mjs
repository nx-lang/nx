/**
 * The one place the site's path prefix is decided.
 *
 * Everything the playground serves lives under `/playground` — the gallery, each example's editor
 * view and every asset — so that the website serves the rest of nxlang.org. Three sides have to
 * agree on the prefix: the Worker script that serves the shell under it, the Vite config that
 * builds asset URLs and output paths under it, and the client. The client reads it back through
 * `import.meta.env.BASE_URL`, which Vite fills from `base`, so the bundle never imports this file.
 */

/** Where the site lives, with no trailing slash: the gallery's own address. */
export const BASE_PATH = "/playground";

/** The shell's `<base href>` and Vite's `base`: the prefix as a directory. */
export const BASE_HREF = `${BASE_PATH}/`;
