import { createElement, type ReactNode } from "react";
import { SkiaLabel, SkiaStack } from "../drawnui/react/index";
import { isAuthoredComponent } from "./evaluate";
import type { InstanceNode, InstanceTree } from "./instances";
import { childrenOf, coerceProps, components, type BindHandler, type NxObject, type NxValue } from "./values";

/**
 * A control the renderer does not know, drawn as itself.
 *
 * Throwing away the whole tree for one unknown tag would hide everything that *is* right, which is
 * the opposite of what a playground is for.
 */
function Placeholder({ type }: { type: string }) {
  return (
    <SkiaStack BackgroundColor="#3d1a1a" Spacing={2.0}>
      <SkiaLabel Text={`⚠ Unknown control "${type}"`} TextColor="#ff8787" FontSize={12.0} />
    </SkiaStack>
  );
}

/** Runs the handler drawn under `token` in `source`'s output with the action a DrawnUI event built. */
export type Dispatch = (source: InstanceNode, token: string, action: NxObject) => void;

/** What the walk needs beyond the value itself. */
export interface DrawContext {
  readonly tree: InstanceTree;
  /** The instance whose rendered output is being drawn, or null for the root function's own value. */
  readonly instance: InstanceNode | null;
  readonly dispatch: Dispatch;
  /** Reported once per unrecognized `$type`, so a gap in the catalog is visible rather than silent. */
  readonly reportUnknown: (type: string) => void;
  /** Reported once per handler no instance can run, as `SkiaButton.onTapped`. */
  readonly reportInert: (where: string) => void;
}

/**
 * Draws the value the root function evaluated to, as one pass over the instance tree: every
 * authored component met is visited, so the instances the pass did not meet are dropped, and every
 * handler the root bound outside any component is reported.
 */
export function drawRoot(value: NxValue, context: DrawContext): ReactNode {
  context.tree.begin();
  const node = drawValue(value, "root", context);
  context.tree.finish();
  for (const where of context.tree.inert) {
    context.reportInert(where);
  }
  return node;
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
    if (typeof type === "string" && isAuthoredComponent(context.tree.program, type)) {
      // A component the author wrote: an instance at this position, drawing what it rendered in
      // its place, with the handlers in its output dispatched against it.
      const instance = context.tree.visit(key, node, context.instance);
      return drawValue(instance.rendered, `${key}/body`, { ...context, instance });
    }
    context.reportUnknown(name);
    return <Placeholder key={key} type={name} />;
  }

  const bind: BindHandler = (event, record, params) => {
    const source = context.instance;
    const token = record.token;
    if (token === undefined || source === null) {
      // Pure evaluation rendered it, so no instance holds a handler under it: nothing can run.
      context.reportInert(`${type}.on${event}`);
      return undefined;
    }
    // The token belongs to the instance this pass drew. Until React commits the next drawing, an
    // event can still reach this callback after a dispatch or a parent redraw replaced it, when
    // the token is retired or, after a re-initialization, names another handler.
    const drawn = source.instance;
    return (_sender, ...args) => {
      if (source.instance !== drawn) {
        return true;
      }
      // The event's arguments become the fields of the action the emit declares, by position;
      // the action's name is the one the runtime rendered on the record.
      const action: Record<string, NxValue | undefined> = { $type: record.action };
      params.forEach((param, index) => {
        action[param] = args[index] as NxValue;
      });
      context.dispatch(source, token, action as NxObject);
      // `ContextMenu` reads `true` as "suppress the browser's own menu"; the other events ignore it.
      return true;
    };
  };
  const children = childrenOf(node).map((child, index) => drawValue(child, `${key}.${index}`, context));
  return createElement(
    type,
    { key, ...coerceProps(node, bind) },
    ...(children.length > 0 ? children : []),
  );
}
