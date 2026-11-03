/**
 * How a property is presented in a row's summary and its inspector: its label, the icon or short
 * label its field starts with, and the group it reads in, from its resolved visualizer or derived
 * from its name.
 */
import type { SourcePropertyVisualizer, SourceTree, SourceVisualizer } from "@nx-lang/language-protocol";

/** Words written in capitals wherever they appear in a derived label. */
const ACRONYMS = new Set(["id", "url", "uri", "ui", "api", "html", "css", "json"]);

/**
 * A label for a property named `name`, as design tools write them: `outerRadius` reads `Outer
 * radius`, `imageURL` reads `Image URL`, `max_length` reads `Max length`, and a boolean's leading
 * `is` or `has` is dropped, so `isRequired` reads `Required`.
 */
export function derivedLabel(name: string, boolean = false): string {
  const words = name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1 $2")
    .split(/[\s_-]+/)
    .filter((word) => word !== "");
  if (boolean && words.length > 1 && /^(is|has)$/i.test(words[0]!)) {
    words.shift();
  }
  return words
    .map((word, position) => {
      const lower = word.toLowerCase();
      if (ACRONYMS.has(lower)) {
        return lower.toUpperCase();
      }
      return position === 0 ? lower.charAt(0).toUpperCase() + lower.slice(1) : lower;
    })
    .join(" ");
}

/** How one property presents itself. */
export interface Presentation {
  readonly name: string;
  readonly label: string;
  readonly icon?: string | undefined;
  readonly short?: string | undefined;
  readonly group?: string | undefined;
}

/** The presentation of `name` under `visualizer`, its label derived when the visualizer gives none. */
export function presentationOf(
  visualizer: SourceVisualizer | undefined,
  name: string,
  boolean = false
): Presentation {
  const found: SourcePropertyVisualizer | undefined = visualizer?.properties?.find((property) => property.name === name);
  return {
    name,
    label: found?.label ?? derivedLabel(name, boolean),
    icon: found?.icon,
    short: found?.short,
    group: found?.group,
  };
}

/** The tooltip of a field and its lead: the label, and the name as source writes it. */
export function tooltipOf(presentation: Presentation): string {
  return presentation.label === presentation.name ? presentation.name : `${presentation.label} · ${presentation.name}`;
}

/** Whether a property's declared type, as spelled, is `boolean`. */
export function isBooleanType(tree: SourceTree, spelled: string | undefined): boolean {
  let name = (spelled ?? "").replace(/[?+*]$/, "").trim();
  for (let depth = 0; depth < 8; depth++) {
    if (name === "boolean") {
      return true;
    }
    const alias = tree.declarations.find((entry) => entry.kind === "alias" && entry.name === name);
    if (alias?.type === undefined) {
      return false;
    }
    name = alias.type.replace(/[?+*]$/, "").trim();
  }
  return false;
}
