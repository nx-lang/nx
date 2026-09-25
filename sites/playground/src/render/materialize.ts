/**
 * Builds DrawnUI controls from an evaluated NX value outside React.
 *
 * <para>The output pane draws through the React reconciler, but a templated list's cells are made
 * by DrawnUI's own factory whenever it decides a cell should exist, which is not a React render.
 * This is the same translation as `drawValue`, applied by hand: the control class from the
 * catalog's metadata, DrawnUI's initial styles, the coerced props, and the children appended in
 * order. An authored component is initialized through the runtime, without a parent instance,
 * and what it rendered is materialized in its place. A handler met here cannot run — a cell is
 * not an instance of the tree — so it is left unbound and reported once as inert.</para>
 */
import { initializeComponent, type NxCanonicalValue } from "@nx-lang/ir-runtime";
import * as DrawnUi from "drawnui-react/core";
import { SkiaLabel, type SkiaControl } from "drawnui-react/core";
import { isAuthoredComponent, type Program } from "./evaluate";
import { stripInertHandlers } from "./instances";
import { childrenOf, coerceProps, components, type BindTemplate, type NxObject, type NxValue } from "./values";

export interface MaterializeContext {
  readonly program: Program;
  /** Reported once per `$type` the catalog does not know. */
  readonly reportUnknown: (type: string) => void;
  /** Reported once per handler that cannot run here, as `SkiaButton.onTapped`. */
  readonly reportInert: (where: string) => void;
  /** Turns a function record on a template property into a cell factory; nested lists stay templated. */
  readonly bindTemplate?: BindTemplate;
}

/** The controls an evaluated value draws as, in order: a list gives one per item, a scalar a label. */
export function materialize(value: NxValue, context: MaterializeContext): SkiaControl[] {
  if (value === null || value === undefined) {
    return [];
  }
  if (Array.isArray(value)) {
    return value.flatMap((item) => materialize(item, context));
  }
  if (typeof value !== "object") {
    const label = new SkiaLabel();
    label.ApplyInitialStyles?.(true);
    label.Text = String(value);
    return [label];
  }
  const node = value as NxObject;
  const type = node.$type;
  if (typeof type !== "string" || components[type] === undefined) {
    const name = typeof type === "string" ? type : "(untyped value)";
    if (typeof type === "string" && isAuthoredComponent(context.program, type)) {
      // The component's props, with any handler the parent bound stripped: no instance of the
      // drawing encloses a cell, so nothing could run it.
      const { value: props, inert } = stripInertHandlers(node);
      for (const where of inert) {
        context.reportInert(where);
      }
      const fields: Record<string, NxCanonicalValue> = {};
      for (const [key, field] of Object.entries(props as NxObject)) {
        if (key !== "$type" && field !== undefined) {
          fields[key] = field as NxCanonicalValue;
        }
      }
      const rendered = initializeComponent(context.program, type, fields).rendered as NxValue;
      return materialize(rendered, context);
    }
    context.reportUnknown(name);
    return [];
  }

  const ctor = controlClass(type);
  if (ctor === undefined) {
    context.reportUnknown(type);
    return [];
  }
  const control = new ctor();
  control.ApplyInitialStyles?.(true);
  const props = coerceProps(
    node,
    (event) => {
      context.reportInert(`${type}.on${event}`);
      return undefined;
    },
    context.bindTemplate,
  );
  applyProps(control, props);
  for (const child of childrenOf(node)) {
    for (const built of materialize(child, context)) {
      control.AddSubView(built);
    }
  }
  return [control];
}

/**
 * The class a tag builds. DrawnUI names each tag's class after the tag, and `drawnui-react/core`
 * exports every one of them, including the layout presets (`SkiaStack`, `SkiaGrid` and the rest) and
 * `TextSpan`. So the package's own `Registry`, which it does not export, is not needed.
 */
function controlClass(type: string): (new () => SkiaControl) | undefined {
  const candidate = (DrawnUi as Record<string, unknown>)[type];
  return typeof candidate === "function" ? (candidate as new () => SkiaControl) : undefined;
}

/** What React never hands a control: the reconciler skips these in its own `applyProps`. */
const RESERVED = new Set(["children", "key", "ref"]);

/**
 * Sets each prop on the control under its own name, as DrawnUI's reconciler does when it creates
 * a control (`applyProps` in `src/react/reconciler.ts`, v0.1.0-preview.12). A new control has no
 * earlier props, so the reconciler's update path does not apply: resetting a removed prop to its
 * default, then `Update` or `RepaintComposition`. A cell is rebuilt, never patched.
 */
function applyProps(control: SkiaControl, props: Record<string, unknown>): void {
  for (const [name, value] of Object.entries(props)) {
    if (!RESERVED.has(name)) {
      (control as unknown as Record<string, unknown>)[name] = value;
    }
  }
}
