/**
 * The site's address scheme, with nothing from the browser in it so it can be tested on its own.
 *
 * <para>Everything is under the prefix. `<prefix>` is the default example, `<prefix>/<id>` is an
 * example, and a `#code=<payload>` fragment carries edited source, which wins over whatever the
 * path names. The server serves the shell for the prefix and one segment below it, so each of
 * these opens directly.</para>
 */
import { payloadFromFragment } from "./share/codec.ts";

/** What an address asks for. */
export type Route =
  /** Source carried in the fragment, still encoded. */
  | { readonly kind: "code"; readonly payload: string }
  /** An example, by id. */
  | { readonly kind: "example"; readonly id: string }
  /** The default example; `missing` names an example id the address asked for and nobody has. */
  | { readonly kind: "default"; readonly missing?: string };

/** What an example id may look like in an address. */
const ID_PATTERN = "[A-Za-z0-9-]+";

function escapeForRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&");
}

/**
 * Resolves an address under `root` to a route.
 *
 * A fragment wins over the path. Otherwise `root` and `root/` are the default example, and
 * `root/<id>` is that example when `isExample` knows the id, or else the default example with a
 * note of what was missing: a stale link lands on something to edit rather than on an error.
 */
export function routeFromLocation(
  path: string,
  hash: string,
  root: string,
  isExample: (id: string) => boolean
): Route {
  const payload = payloadFromFragment(hash);
  if (payload !== null) {
    return { kind: "code", payload };
  }
  const match = new RegExp(`^${escapeForRegExp(root)}(?:/(${ID_PATTERN}))?/?$`).exec(path);
  const id = match?.[1];
  if (id === undefined) {
    return { kind: "default" };
  }
  return isExample(id) ? { kind: "example", id } : { kind: "default", missing: id };
}

/** The address of an example. */
export function pathForExample(id: string, root: string): string {
  return `${root}/${id}`;
}

/** The address of edited source: the prefix and the fragment, whatever example it began as. */
export function pathForPayload(payload: string, root: string): string {
  return `${root}#code=${payload}`;
}
