import { useEffect, useState } from "react";

/** Where the app is: the gallery, or one example's fiddle. */
export type Route = { readonly kind: "gallery" } | { readonly kind: "fiddle"; readonly id: string };

export function routeFromPath(path: string): Route {
  const match = /^\/fiddle\/([A-Za-z0-9-]+)\/?$/.exec(path);
  return match === null ? { kind: "gallery" } : { kind: "fiddle", id: match[1]! };
}

export function pathForRoute(route: Route): string {
  return route.kind === "gallery" ? "/" : `/fiddle/${route.id}`;
}

/**
 * The app's whole router.
 *
 * Two addresses and the browser's own history are all this needs; a routing library would be more
 * machinery than the app has routes. The server serves the shell for any address it does not
 * recognize, so `/fiddle/shapes` opens directly.
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
