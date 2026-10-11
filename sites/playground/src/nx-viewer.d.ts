import type { NxViewerElement } from "@nx-lang/viewer";
import type { DetailedHTMLProps, HTMLAttributes } from "react";

// React 19 sets these as properties on the element, since the element defines them.
declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "nx-viewer": DetailedHTMLProps<HTMLAttributes<NxViewerElement>, NxViewerElement> &
        Partial<Pick<NxViewerElement, "stale" | "ghosts">>;
    }
  }
}
