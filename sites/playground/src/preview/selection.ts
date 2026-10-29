/** Shared source/output selection contract. Renderer IDs are scoped to their current output. */
export interface SourceSite {
  readonly module: string;
  readonly start: number;
  readonly end: number;
}
export interface SourceProvenance extends SourceSite {
  readonly callSites?: readonly SourceSite[];
}
export interface SourceSelection {
  /** The exact source snapshot these UTF-8 byte offsets index. */
  readonly source: string;
  readonly start: number;
  readonly end: number;
}
export interface PreviewSelectionTarget {
  readonly id: string;
  readonly sources: readonly SourceSite[];
}
/** Prefer the construction site when editable, otherwise the innermost authored helper call. */
export function authoredSites(
  origin: SourceProvenance | null | undefined,
  module: string,
): readonly SourceSite[] {
  return origin == null
    ? []
    : [origin, ...(origin.callSites ?? [])].filter(
        (site) => site.module === module,
      );
}
/** Shared by canvas, structured records and future domain-specific preview adapters. */
export function selectedTargets(
  targets: readonly PreviewSelectionTarget[],
  range: SourceSelection | null,
  module: string,
): string[] {
  if (range === null) {
    return [];
  }
  const sites = targets.map((target) => ({
    ...target,
    sources: target.sources.filter((site) => site.module === module),
  }));
  const contained = sites.filter((target) =>
    target.sources.some(
      (site) => site.start >= range.start && site.end <= range.end,
    ),
  );
  if (contained.length > 0) {
    return contained.map((target) => target.id);
  }
  // A property/value selected inside an element selects its nearest renderings, not every parent.
  const enclosing = sites.flatMap((target) =>
    target.sources
      .filter((site) => site.start <= range.start && site.end >= range.end)
      .map((site) => ({ id: target.id, size: site.end - site.start })),
  );
  const smallest = enclosing.reduce(
    (size, site) => Math.min(size, site.size),
    Infinity,
  );
  return [
    ...new Set(
      enclosing.filter((site) => site.size === smallest).map((site) => site.id),
    ),
  ];
}
