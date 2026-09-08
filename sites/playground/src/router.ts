import { useEffect, useState } from "react";
import { exampleById } from "./examples";
import { SITE_ROOT } from "./paths";
import { type Route, pathForRoute as pathForRouteUnder, routeFromPath as routeFromPathUnder } from "./routes";

export type { Route } from "./routes";

const isExample = (id: string) => exampleById(id) !== undefined;

/** The site's address scheme, bound to where this build lives and to the example set it carries. */
export function routeFromPath(path: string): Route {
  return routeFromPathUnder(path, SITE_ROOT, isExample);
}

export function pathForRoute(route: Route): string {
  return pathForRouteUnder(route, SITE_ROOT);
}

/**
 * The app's whole router.
 *
 * Two addresses and the browser's own history are all this needs; a routing library would be more
 * machinery than the app has routes. The server serves the shell for any address under the prefix
 * that names no file, so `/playground/shapes` opens directly.
 */
export function useRoute(): [Route, (route: Route) => void] {
  const [route, setRoute] = useState<Route>(() => routeFromPath(window.location.pathname));

  useEffect(() => {
    const onPop = () => setRoute(routeFromPath(window.location.pathname));
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const navigate = (next: Route) => {
    window.history.pushState(null, "", pathForRoute(next));
    setRoute(next);
  };

  return [route, navigate];
}
