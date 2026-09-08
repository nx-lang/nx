import type { SkiaControl } from "./SkiaControl";
import type { RecyclingTemplate } from "./Types";

/**
 * Mirrors DrawnUi ViewsAdapter (ChildrenFactory): creates template instances for item indexes and,
 * with RecyclingTemplate.Enabled, keeps a pool so a view leaving the visible range is rebound to the
 * next index that needs one instead of being created again.
 */
export class ViewsAdapter {
  private readonly inUse = new Map<number, SkiaControl>();
  private readonly pool: SkiaControl[] = [];
  private template?: () => SkiaControl;
  private items: readonly unknown[] = [];
  private recycling: RecyclingTemplate = "Enabled";
  /** Views created so far (diagnostics). */
  Created = 0;

  constructor(private readonly parent: SkiaControl) {}

  get PoolSize(): number { return this.pool.length; }
  get InUseCount(): number { return this.inUse.size; }

  /** New template / items / mode: every realized view is dropped (DrawnUi ApplyItemsSource). */
  Initialize(template: (() => SkiaControl) | undefined, items: readonly unknown[], recycling: RecyclingTemplate): void {
    this.template = template;
    this.items = items;
    this.recycling = recycling;
    for (const v of this.inUse.values()) v.Parent = undefined;
    this.inUse.clear();
    this.pool.length = 0;
  }

  /** Items array changed but the template did not: keep views, they get rebound on next use. */
  UpdateItems(items: readonly unknown[]): void {
    this.items = items;
    for (const [index, view] of this.inUse) {
      if (index >= items.length) { this.ReleaseViewAt(index); continue; }
      if (view.BindingContext !== items[index]) view.BindingContext = items[index];
    }
  }

  GetExistingViewAtIndex(index: number): SkiaControl | undefined { return this.inUse.get(index); }

  /** Items were inserted at the head: realized views keep their item, only their index moves (no rebind). */
  ShiftIndices(by: number, items: readonly unknown[]): void {
    this.items = items;
    const moved = new Map<number, SkiaControl>();
    for (const [index, view] of this.inUse) moved.set(index + by, view);
    this.inUse.clear();
    for (const [index, view] of moved) this.inUse.set(index, view);
  }

  /** View bound to items[index]: existing, recycled from the pool, or freshly created. */
  GetOrCreateViewForIndex(index: number): SkiaControl | undefined {
    const existing = this.inUse.get(index);
    if (existing) return existing;
    if (!this.template || index < 0 || index >= this.items.length) return undefined;
    let view = this.recycling === "Enabled" ? this.pool.pop() : undefined;
    if (!view) { view = this.template(); this.Created++; }
    view.Parent = this.parent;
    view.ContextIndex = index;
    view.BindingContext = this.items[index];
    this.inUse.set(index, view);
    return view;
  }

  /** Index left the visible range: recycled views go back to the pool, non-recycled ones stay realized. */
  ReleaseViewAt(index: number): void {
    const view = this.inUse.get(index);
    if (!view) return;
    if (this.recycling !== "Enabled") return; // Disabled = one view per item, kept alive
    this.inUse.delete(index);
    view.Parent = undefined; // detached until rebound: keeps pooled cells out of gestures and the accessibility tree
    this.pool.push(view);
  }

  /** Releases every realized index outside [first, last]. */
  ReleaseOutside(first: number, last: number): void {
    if (this.recycling !== "Enabled") return;
    for (const index of [...this.inUse.keys()]) if (index < first || index > last) this.ReleaseViewAt(index);
  }

  /** Views currently bound, in index order (gesture listeners / drawing). */
  GetViewsInUse(): SkiaControl[] {
    return [...this.inUse.entries()].sort((a, b) => a[0] - b[0]).map((e) => e[1]);
  }
}
