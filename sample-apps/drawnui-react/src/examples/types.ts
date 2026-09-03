/**
 * How completely an example covers the DrawnUI original it was ported from.
 *
 * Three states rather than two, because most of these ports *work*: they draw correctly and
 * completely, and what is missing is motion. Calling that "partial" would present a correct port as
 * a faulty one, and with most of the set flagged the gallery would read as broken software.
 */
export type Coverage = "complete" | "static" | "reduced";

/**
 * The capabilities NX does not have yet, named once and shared.
 *
 * A fixed vocabulary rather than per-example prose: prose cannot be counted, drifts in wording
 * across a dozen cards, and cannot be searched. Tags make the gallery a coverage report — "four
 * examples need event handlers" is a roadmap signal — and when a capability lands, one search names
 * every example ready to be upgraded.
 */
export type Capability = "event-handlers" | "animation" | "component-state" | "list-virtualization";

/** The one place each capability is worded, so two examples sharing a tag word it identically. */
export const CAPABILITY_WORDING: Record<Capability, string> = {
  "event-handlers": "event handlers",
  animation: "animation",
  "component-state": "component state",
  "list-virtualization": "list virtualization",
};

export interface Example {
  /** The address this example lives at: `/fiddle/<id>`. */
  readonly id: string;
  /** The name the DrawnUI demo site gives it. */
  readonly name: string;
  readonly blurb: string;
  readonly coverage: Coverage;
  /** Empty for a complete port; at least one capability otherwise. */
  readonly capabilities: readonly Capability[];
  /**
   * What the original demonstrates that this port does not. Reduced examples only — a static
   * example's gap is fully described by its capabilities.
   */
  readonly demonstrates?: string;
  readonly source: string;
}

/**
 * The sentence shown beside an example, derived from its state and tags rather than written per
 * example.
 */
export function coverageNote(example: Example): string | null {
  const missing = example.capabilities.map((name) => CAPABILITY_WORDING[name]);
  const list =
    missing.length <= 1
      ? missing.join("")
      // "or", not "and": the sentence is about what NX does not have.
      : `${missing.slice(0, -1).join(", ")} or ${missing[missing.length - 1]}`;
  switch (example.coverage) {
    case "complete":
      return null;
    case "static":
      return `Drawn as in the original. NX has no ${list} yet, so nothing here responds.`;
    case "reduced":
      return `Scaled down: the original demonstrates ${example.demonstrates ?? "more than this"}, which needs ${list}.`;
  }
}
