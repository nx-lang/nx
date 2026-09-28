import type { NxValueElement } from "@nx-lang/value-view";
import type { DetailedHTMLProps, HTMLAttributes } from "react";

// React 19 sets these as properties on the element, since the element defines them.
declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "nx-value": DetailedHTMLProps<HTMLAttributes<NxValueElement>, NxValueElement> &
        Partial<Pick<NxValueElement, "value" | "stale" | "truncated" | "describe" | "highlighter">>;
    }
  }
}
