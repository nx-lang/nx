/**
 * How completely an example covers the DrawnUI original it was ported from.
 *
 * Three states rather than two, because most of these ports *work*: they draw correctly and
 * completely, and what is missing is motion. Calling that "partial" would present a correct port as
 * a faulty one, and with most of the set flagged the gallery would read as broken software.
 */
export type Coverage = "complete" | "static" | "reduced";

/**
 * The capabilities an example's gap is attributed to, named once and shared.
 *
 * A fixed vocabulary rather than per-example prose: prose cannot be counted, drifts in wording
 * across a dozen cards, and cannot be searched. Tags make the gallery a coverage report — "four
 * examples need event handlers" is a roadmap signal — and when a capability lands, one search names
 * every example ready to be upgraded.
 *
 * Two of them, `event-handlers` and `component-state`, NX now has: a tag naming one says the port
 * does not use it yet, which is porting work, not a language gap. The other three NX lacks.
 *
 * `code-behind` is an engine object built or driven from code and attached to a control — a
 * shader effect in `VisualEffects`, a CanvasKit filter in `PaintColorFilter`, a `SkiaSpriteSet`
 * subclass, a cell class carrying drag logic. It is what DrawnUI's own docs call code-behind, and
 * none of the other tags names it: the missing piece is neither a handler nor state but an object
 * NX has no way to construct. It also names the other direction — a handler calling into a control
 * (`Seek(30)`, `SelectAll()`) or reading from one (a Lottie's frame count, the accessibility
 * manager's snapshot) — since NX has no way to reach the object either.
 */
export type Capability =
  | "event-handlers"
  | "animation"
  | "component-state"
  | "list-virtualization"
  | "code-behind";

/** The one place each capability is worded, so two examples sharing a tag word it identically. */
export const CAPABILITY_WORDING: Record<Capability, string> = {
  "event-handlers": "event handlers",
  animation: "animation",
  "component-state": "component state",
  "list-virtualization": "list virtualization",
  "code-behind": "code-behind",
};

/** Whether NX lacks the capability, or has it and the port does not use it yet. */
export const CAPABILITY_STATUS: Record<Capability, "missing" | "unused"> = {
  "event-handlers": "unused",
  animation: "missing",
  "component-state": "unused",
  "list-virtualization": "missing",
  "code-behind": "missing",
};

export interface Example {
  /** The address this example's editor view lives at: `/playground/<id>`. */
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
 * example, and addressed to a visitor: it says what the example is, not what a maintainer owes.
 */
export function coverageNote(example: Example): string | null {
  switch (example.coverage) {
    case "complete":
      return null;
    case "static":
      return `Static: drawn as in the original, but some of its motion or interaction is absent. ${describeGaps(example.capabilities)}`;
    case "reduced":
      return `Reduced: the original demonstrates ${example.demonstrates ?? "more than this"}. ${describeGaps(example.capabilities)}`;
  }
}

/** The capabilities as a list: "animation, code-behind or event handlers" — "or", since each is absent. */
function listOf(capabilities: readonly Capability[]): string {
  const words = capabilities.map((name) => CAPABILITY_WORDING[name]);
  return words.length <= 1 ? words.join("") : `${words.slice(0, -1).join(", ")} or ${words[words.length - 1]}`;
}

/**
 * The sentence that separates what NX lacks from what the port has not used: "NX has no animation
 * yet, and this port does not use event handlers yet." A landed capability is never presented as
 * missing from NX.
 */
function describeGaps(capabilities: readonly Capability[]): string {
  const missing = capabilities.filter((name) => CAPABILITY_STATUS[name] === "missing");
  const unused = capabilities.filter((name) => CAPABILITY_STATUS[name] === "unused");
  const lacks = missing.length === 0 ? null : `NX has no ${listOf(missing)} yet`;
  const skips = unused.length === 0 ? null : `this port does not use ${listOf(unused)} yet`;
  if (lacks !== null && skips !== null) {
    return `${lacks}, and ${skips}.`;
  }
  const only = lacks ?? skips;
  return only === null ? "" : `${only.charAt(0).toUpperCase()}${only.slice(1)}.`;
}
