import type { DrawingContext } from "../core/SkiaControl";
import { type SkiaCacheType, SKRect, type TransformAspect } from "../core/Types";
import { SkiaImage } from "./SkiaImage";

/**
 * Mirrors DrawnUi SkiaImageTiles: the loaded source is drawn as a cached tile of `TileWidth` x `TileHeight` points
 * (`TileAspect` inside the tile, default AspectCover) repeated over the box, shifted by `TileOffsetX` / `TileOffsetY`
 * (points, wrap around — animate them for a scrolling background). Nothing is drawn until both tile sizes are set.
 */
export class SkiaImageTiles extends SkiaImage {
  TileAspect: TransformAspect = "AspectCover";
  TileWidth = 0;
  TileHeight = 0;
  TileOffsetX = 0;
  TileOffsetY = 0;
  /** Cache of the tile image (C# default Image). */
  TileCacheType: SkiaCacheType = "Image";

  /** Cached image used as the tile (C# Tile). */
  protected Tile?: SkiaImage;

  constructor() {
    super();
    this.IsClippedToBounds = true; // C# WillClipBounds
  }

  /** C# CreateTile: a SkiaImage sharing the decoded source, sized to one tile. */
  protected CreateTile(width: number, height: number): SkiaImage {
    const tile = new SkiaImage();
    tile.Aspect = this.TileAspect;
    tile.WidthRequest = width; tile.HeightRequest = height;
    tile.HorizontalOptions = "Fill"; tile.VerticalOptions = "Fill";
    tile.UseCache = this.TileCacheType;
    tile.Parent = this;
    return tile;
  }

  private tileKey = "";
  /** C# SetupTiles: (re)creates the tile when the source or the tile geometry changed. */
  protected SetupTiles(): void {
    const image = this.LoadedSource;
    const key = `${this.TileWidth}|${this.TileHeight}|${this.TileAspect}|${this.TileCacheType}`;
    if (this.TileWidth > 0 && this.TileHeight > 0 && image) {
      if (!this.Tile || this.tileKey !== key) { this.Tile?.Dispose(); this.Tile = this.CreateTile(this.TileWidth, this.TileHeight); this.tileKey = key; }
      if (this.Tile.ImageBitmap !== image) this.Tile.ImageBitmap = image;
    }
  }

  protected override Paint(ctx: DrawingContext): void {
    this.SetupTiles();
    const tile = this.Tile;
    if (!tile || !this.LoadedSource) return;
    const dest = ctx.Destination, scale = ctx.Scale;
    const tw = Math.round(this.TileWidth * scale), th = Math.round(this.TileHeight * scale);
    if (tw <= 0 || th <= 0) return;
    // C# DrawSource: offsets wrap inside one tile, the grid starts one tile before the box when shifted
    const useOffsetX = -this.TileOffsetX % this.TileWidth, useOffsetY = -this.TileOffsetY % this.TileHeight;
    const offsetX = useOffsetX > 0 ? Math.round(useOffsetX * scale) : tw + Math.round(useOffsetX * scale);
    const offsetY = useOffsetY > 0 ? Math.round(useOffsetY * scale) : th + Math.round(useOffsetY * scale);
    const tilesX = Math.ceil((dest.Width + offsetX) / tw), tilesY = Math.ceil((dest.Height + Math.abs(offsetY)) / th);
    const startX = dest.Left - offsetX, startY = dest.Top - offsetY;
    this.DisplayRect = dest;
    for (let x = 0; x < tilesX; x++) {
      for (let y = 0; y < tilesY; y++) {
        const left = startX + x * tw, top = startY + y * th;
        const r = new SKRect(left, top, left + tw, top + th);
        tile.Measure(tw, th, scale);
        tile.Arrange(r, tile.WidthRequest, tile.HeightRequest, scale);
        tile.Render(ctx);
      }
    }
  }

  protected override OnDisposing(): void { this.Tile?.Dispose(); this.Tile = undefined; super.OnDisposing(); }
}
