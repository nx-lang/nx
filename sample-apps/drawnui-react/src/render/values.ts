import meta from "../../catalog/catalog-meta.json";
import { CornerRadius, SkiaPoint, SkiaShadow, Thickness } from "../drawnui/index";

/** Anything the IR runtime can hand back. */
export type NxValue = string | number | boolean | null | NxValue[] | NxObject;
export interface NxObject {
  readonly $type?: string;
  readonly [key: string]: NxValue | undefined;
}

/**
 * Turns a record value into the object DrawnUI expects.
 *
 * Four of the catalog's five records are DrawnUI classes, and DrawnUI checks `instanceof` in
 * places, so a plain object is not interchangeable with them. Each entry is written out rather than
 * derived from the field order, because the constructors do not agree on a shape: `CornerRadius`
 * takes four positions and fills the missing ones from the first, while `SkiaShadow` takes an
 * object. A record that gains a constructor in the catalog and not here fails loudly below, which
 * is the intended behavior — silently passing a plain object would draw something subtly wrong.
 */
const CONSTRUCTORS: Record<string, (fields: Record<string, unknown>) => unknown> = {
  Thickness: (fields) =>
    new Thickness(
      (fields.Left as number) ?? 0,
      (fields.Top as number) ?? 0,
      (fields.Right as number) ?? 0,
      (fields.Bottom as number) ?? 0,
    ),
  // Undefined rather than 0 for the unset corners: the constructor mirrors TopLeft into them,
  // which is what `CornerRadius={12}` means in the DrawnUI original.
  CornerRadius: (fields) =>
    new CornerRadius(
      (fields.TopLeft as number) ?? 0,
      fields.TopRight as number | undefined,
      fields.BottomLeft as number | undefined,
      fields.BottomRight as number | undefined,
    ),
  SkiaPoint: (fields) => new SkiaPoint((fields.X as number) ?? 0, (fields.Y as number) ?? 0),
  SkiaShadow: (fields) => new SkiaShadow(fields as Partial<SkiaShadow>),
};

const unions = meta.unions as Record<string, readonly string[]>;
const records = meta.records as Record<string, { construct: string | null; fields: readonly string[] }>;
export const components = meta.components as Record<string, { class: string; content: string | null }>;
export const contentProperty = meta.contentProperty;

function isObject(value: NxValue): value is NxObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Coerces one evaluated NX value into what DrawnUI expects for a property.
 *
 * The tags are read from the generated metadata rather than inferred from the value's shape. A
 * union case's discriminator happens to contain a dot and a record's does not, so shape would
 * mostly work — but the record-to-constructor mapping has to exist regardless, and one mechanism
 * that is always right beats two that are usually right.
 */
export function coerce(value: NxValue): unknown {
  if (Array.isArray(value)) {
    return value.map(coerce).filter((item) => item !== undefined);
  }
  if (!isObject(value)) {
    return value;
  }

  const type = value.$type;
  if (typeof type === "string") {
    const dot = type.indexOf(".");
    if (dot > 0 && unions[type.slice(0, dot)] !== undefined) {
      // The Rust interpreter writes a union case as its bare name and the TypeScript runtime writes
      // `{ $type: "LayoutOptions.Center" }`. The fiddle evaluates through the second, and DrawnUI
      // wants the first.
      return type.slice(dot + 1);
    }
    const record = records[type];
    if (record !== undefined) {
      const fields: Record<string, unknown> = {};
      for (const name of record.fields) {
        const field = value[name];
        if (field !== null && field !== undefined) {
          fields[name] = coerce(field);
        }
      }
      const construct = record.construct === null ? null : CONSTRUCTORS[record.construct];
      if (record.construct !== null && construct === undefined) {
        throw new Error(
          `The catalog says '${type}' is constructed as '${record.construct}', but the renderer has no constructor for it.`,
        );
      }
      return construct === null ? fields : construct(fields);
    }
  }
  return value;
}

/** The properties of a control, coerced, with nulls dropped so DrawnUI's own defaults survive. */
export function coerceProps(value: NxObject): Record<string, unknown> {
  const props: Record<string, unknown> = {};
  for (const [name, item] of Object.entries(value)) {
    if (name === "$type" || name === contentProperty || item === null || item === undefined) {
      continue;
    }
    props[name] = coerce(item);
  }
  return props;
}

/** The children of a control: always a list, or none. */
export function childrenOf(value: NxObject): readonly NxValue[] {
  const content = value[contentProperty];
  if (content === null || content === undefined) {
    return [];
  }
  return Array.isArray(content) ? content : [content];
}
