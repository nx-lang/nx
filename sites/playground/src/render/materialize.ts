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
import type { SkiaControl } from "../drawnui/core/SkiaControl";
import { SkiaLabel } from "../drawnui/controls/SkiaLabel";
import { Registry, applyProps } from "../drawnui/react/registry";
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

  const ctor = Registry[type];
  if (ctor === undefined) {
    context.reportUnknown(type);
    return [];
  }
  const control = new ctor() as SkiaControl;
  control.ApplyInitialStyles?.(true);
  const props = coerceProps(
    node,
    (event) => {
      context.reportInert(`${type}.on${event}`);
      return undefined;
    },
    context.bindTemplate,
  );
  applyProps(control, null, props);
  for (const child of childrenOf(node)) {
    for (const built of materialize(child, context)) {
      control.AddSubView(built);
    }
  }
  return [control];
}
