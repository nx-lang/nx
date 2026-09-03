import type { Image } from "canvaskit-wasm";
import { Super } from "./Super";

/**
 * Mirrors DrawnUi SkiaImageManager (subset): loads + decodes image sources once and shares the
 * decoded CanvasKit Image between all SkiaImage instances using the same source.
 * Sources are URLs (absolute, or relative to the page like "images/photo.jpg").
 */
export class SkiaImageManager {
  static readonly Instance = new SkiaImageManager();

  /** Keep decoded images for reuse across controls (DrawnUi ReuseBitmaps). */
  static ReuseBitmaps = true;

  private readonly cache = new Map<string, Image>();
  private readonly pending = new Map<string, Promise<Image>>();

  /** Decoded image for a source, from cache or after fetch + decode. Rejects on network/decode failure. */
  LoadImageAsync(source: string): Promise<Image> {
    const cached = this.cache.get(source);
    if (cached) return Promise.resolve(cached);
    let inflight = this.pending.get(source);
    if (inflight) return inflight;
    inflight = (async () => {
      const response = await fetch(source);
      if (!response.ok) throw new Error(`DrawnUi: ${response.status} loading image '${source}'`);
      const image = Super.CK.MakeImageFromEncoded(await response.arrayBuffer());
      if (!image) throw new Error(`DrawnUi: cannot decode image '${source}'`);
      if (SkiaImageManager.ReuseBitmaps) this.cache.set(source, image);
      return image;
    })();
    this.pending.set(source, inflight);
    inflight.finally(() => this.pending.delete(source));
    return inflight;
  }

  /** Warms the cache so first paint of these images is instant (DrawnUi PreloadImages). Failures are ignored. */
  async PreloadImages(sources: string[]): Promise<void> {
    await Promise.all(sources.map((s) => this.LoadImageAsync(s).catch(() => undefined)));
  }

  /** Drops a cached image (or everything) — decoded images hold GPU/WASM memory. */
  Clear(source?: string): void {
    if (source) { this.cache.get(source)?.delete(); this.cache.delete(source); return; }
    for (const img of this.cache.values()) img.delete();
    this.cache.clear();
  }
}
