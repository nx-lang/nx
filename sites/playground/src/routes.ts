/**
 * The site's address scheme, with nothing from the browser in it so it can be tested on its own.
 *
 * Two addresses under the prefix: the gallery at the prefix itself, and an example's editor view
 * at `<prefix>/<id>`. The server serves the shell for both, so either opens directly.
 */

/** Where the app is: the gallery, or one example's editor view. */
export type Route = { readonly kind: "gallery" } | { readonly kind: "editor"; readonly id: string };

/** What an example id may look like in an address. */
const ID_PATTERN = "[A-Za-z0-9-]+";

function escapeForRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&");
}

/**
 * Resolves an address under `root` to a route.
 *
 * `root` and `root/` are the gallery. `root/<id>` is that example's editor view when `isExample`
 * knows the id, and the gallery otherwise: a stale link lands on the front page rather than on an
 * error, and the front page is where every example can be found again.
 */
export function routeFromPath(path: string, root: string, isExample: (id: string) => boolean): Route {
  const match = new RegExp(`^${escapeForRegExp(root)}(?:/(${ID_PATTERN}))?/?$`).exec(path);
  const id = match?.[1];
  return id !== undefined && isExample(id) ? { kind: "editor", id } : { kind: "gallery" };
}

export function pathForRoute(route: Route, root: string): string {
  return route.kind === "gallery" ? root : `${root}/${route.id}`;
}
