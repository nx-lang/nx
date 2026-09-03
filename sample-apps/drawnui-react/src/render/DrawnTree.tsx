import { createElement, type ReactNode } from "react";
import { SkiaLabel, SkiaStack } from "../drawnui/react/index";
import { isAuthoredComponent, renderAuthoredComponent, type Program } from "./evaluate";
import { childrenOf, coerceProps, components, type NxObject, type NxValue } from "./values";

/** Reported once per unrecognized `$type`, so a gap in the catalog is visible rather than silent. */
export type ReportUnknown = (type: string) => void;

/**
 * A control the renderer does not know, drawn as itself.
 *
 * Throwing away the whole tree for one unknown tag would hide everything that *is* right, which is
 * the opposite of what a fiddle is for.
 */
function Placeholder({ type }: { type: string }) {
  return (
    <SkiaStack BackgroundColor="#3d1a1a" Spacing={2.0}>
      <SkiaLabel Text={`⚠ Unknown control "${type}"`} TextColor="#ff8787" FontSize={12.0} />
    </SkiaStack>
  );
}

/** What the walk needs beyond the value itself. */
export interface DrawContext {
  readonly program: Program;
  readonly report: ReportUnknown;
}

/** Walks an evaluated NX value tree, instantiating DrawnUI controls through the reconciler. */
export function drawValue(value: NxValue, key: string, context: DrawContext): ReactNode {
  if (value === null || value === undefined) {
    return null;
  }
  if (Array.isArray(value)) {
    return value.map((item, index) => drawValue(item, `${key}.${index}`, context));
  }
  if (typeof value !== "object") {
    // A bare value in content position is text; the closest DrawnUI equivalent is a label.
    return <SkiaLabel key={key} Text={String(value)} />;
  }

  const node = value as NxObject;
  const type = node.$type;
  if (typeof type !== "string" || components[type] === undefined) {
    const name = typeof type === "string" ? type : "(untyped value)";
    if (typeof type === "string" && isAuthoredComponent(context.program, type)) {
      // A component the author wrote: draw what it renders, in its place.
      return drawValue(renderAuthoredComponent(context.program, type, node), key, context);
    }
    context.report(name);
    return <Placeholder key={key} type={name} />;
  }

  const children = childrenOf(node).map((child, index) => drawValue(child, `${key}.${index}`, context));
  return createElement(
    type,
    { key, ...coerceProps(node) },
    ...(children.length > 0 ? children : []),
  );
}
