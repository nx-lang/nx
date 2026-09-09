import type { Image } from "canvaskit-wasm";
import { Super } from "./Super";

/** C# LoadPriority: order of the managed load queue. */
export type LoadPriority = "Low" | "Normal" | "High";

interface QueueItem { source: string; priority: LoadPriority; signal?: AbortSignal; resolve: (i: Image) => void; reject: (e: Error) => void }

const PRIORITY_RANK: Record<LoadPriority, number> = { High: 0, Normal: 1, Low: 2 };

/**
 * Mirrors DrawnUi SkiaImageManager: loads + decodes image sources once and shares the decoded CanvasKit Image
 * between all SkiaImage instances using the same source (`ReuseBitmaps`). `LoadImageManagedAsync` goes through the
 * priority queue (High > Normal > Low, at most `MaxParallelLoads` fetches in flight, C# semaphore of 5; the same
 * source requested twice is fetched once), `LoadImageAsync` loads directly. `PreloadImages` warms the cache,
 * `CancelAll` drops what is still queued. Sources are URLs (absolute, or relative to the page like "images/photo.jpg").
 */
export class SkiaImageManager {
  static readonly Instance = new SkiaImageManager();

  /** Keep decoded images for reuse across controls (DrawnUi ReuseBitmaps). */
  static ReuseBitmaps = true;
  /** Fetches in flight at once through the managed queue (C# CreateSemaphoreForLocalFiles: 5). */
  static MaxParallelLoads = 5;
  /** C# CanReload: raised when a queued load failed so the caller may retry later. */
  CanReload?: (source: string, error: Error) => void;

  private readonly cache = new Map<string, Image>();
  private readonly pending = new Map<string, Promise<Image>>();
  private readonly queue: QueueItem[] = [];
  private readonly waiting = new Map<string, QueueItem[]>();
  private running = 0;

  /** Queue diagnostics. */
  get QueuedCount(): number { return this.queue.length; }
  get RunningCount(): number { return this.running; }

  /** Cached decoded image, if loaded already (C# GetFromCache). */
  GetFromCache(source: string): Image | undefined { return this.cache.get(source); }

  /** Direct load without the queue (C# LoadImageAsync); cached and de-duplicated. Rejects on network/decode failure. */
  LoadImageAsync(source: string, signal?: AbortSignal): Promise<Image> {
    const cached = this.cache.get(source);
    if (cached) return Promise.resolve(cached);
    let inflight = this.pending.get(source);
    if (inflight) return inflight;
    inflight = this.Fetch(source, signal);
    this.pending.set(source, inflight);
    inflight.finally(() => this.pending.delete(source)).catch(() => undefined);
    return inflight;
  }

  private async Fetch(source: string, signal?: AbortSignal): Promise<Image> {
    const response = await fetch(source, signal ? { signal } : undefined);
    if (!response.ok) throw new Error(`DrawnUi: ${response.status} loading image '${source}'`);
    const image = Super.CK.MakeImageFromEncoded(await response.arrayBuffer());
    if (!image) throw new Error(`DrawnUi: cannot decode image '${source}'`);
    if (SkiaImageManager.ReuseBitmaps) this.cache.set(source, image);
    return image;
  }

  /** Queued load (C# LoadImageManagedAsync): cached images resolve at once, the rest wait their turn by priority. */
  LoadImageManagedAsync(source: string, signal?: AbortSignal, priority: LoadPriority = "Normal"): Promise<Image> {
    const cached = this.cache.get(source);
    if (cached) return Promise.resolve(cached);
    const inflight = this.pending.get(source);
    if (inflight) return inflight;
    return new Promise<Image>((resolve, reject) => {
      const item: QueueItem = { source, priority, signal, resolve, reject };
      const same = this.waiting.get(source);
      if (same) { same.push(item); return; } // already queued: one fetch serves every requester
      this.waiting.set(source, [item]);
      const rank = PRIORITY_RANK[priority];
      let at = this.queue.length;
      while (at > 0 && PRIORITY_RANK[this.queue[at - 1].priority] > rank) at--;
      this.queue.splice(at, 0, item);
      this.Pump();
    });
  }

  private Pump(): void {
    while (this.running < SkiaImageManager.MaxParallelLoads && this.queue.length > 0) {
      const item = this.queue.shift()!;
      const requesters = this.waiting.get(item.source) ?? [item];
      this.waiting.delete(item.source);
      if (item.signal?.aborted) { for (const r of requesters) r.reject(new Error("DrawnUi: image load cancelled")); continue; }
      this.running++;
      const task = this.LoadImageAsync(item.source, item.signal);
      task.then(
        (image) => { for (const r of requesters) r.resolve(image); },
        (error: Error) => { this.CanReload?.(item.source, error); for (const r of requesters) r.reject(error); },
      ).finally(() => { this.running--; this.Pump(); });
    }
  }

  /** C# PreloadImage: warms the cache for one source; failures are ignored. */
  async PreloadImage(source: string, priority: LoadPriority = "Normal"): Promise<void> {
    await this.LoadImageManagedAsync(source, undefined, priority).catch(() => undefined);
  }

  /** Warms the cache so first paint of these images is instant (DrawnUi PreloadImages). Failures are ignored. */
  async PreloadImages(sources: string[], priority: LoadPriority = "Normal"): Promise<void> {
    await Promise.all(sources.map((s) => this.PreloadImage(s, priority)));
  }

  /** Drops everything still waiting in the queue (C# CancelAll); loads already in flight finish. */
  CancelAll(): void {
    const items = this.queue.splice(0);
    for (const item of items) {
      const requesters = this.waiting.get(item.source) ?? [item];
      this.waiting.delete(item.source);
      for (const r of requesters) r.reject(new Error("DrawnUi: image load cancelled"));
    }
  }

  /** Drops a cached image (or everything) — decoded images hold GPU/WASM memory. */
  Clear(source?: string): void {
    if (source) { this.cache.get(source)?.delete(); this.cache.delete(source); return; }
    for (const img of this.cache.values()) img.delete();
    this.cache.clear();
  }
}
