import type { SKRect } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";
import { SkiaSprite, type SpritePlacementConfig } from "./SkiaSprite";

/**
 * Mirrors DrawnUi SkiaSpriteSet: a stateful sprite switcher that pre-creates one SkiaSprite per integer `State`
 * through `Define()`; setting `State` swaps the active child atomically (`OnChangeState`, override it and call
 * super, then adjust `CurrentSprite`, e.g. ScaleX for mirroring). Sprites fill the set's box.
 */
export class SkiaSpriteSet extends SkiaLayout {
  private readonly sprites = new Map<number, SkiaSprite>();
  private active?: SkiaSprite;
  private state = 0;

  constructor() {
    super();
    this.Type = "Absolute";
    this.UseCache = "Operations";
  }

  /** Current integer state; setting it triggers OnChangeState. */
  get State(): number { return this.state; }
  set State(v: number) { if (this.state !== v) { const old = this.state; this.state = v; this.OnChangeState(old, v); this.Update(); } }

  /** The currently active SkiaSprite. */
  get CurrentSprite(): SkiaSprite | undefined { return this.active; }

  /** Creates and registers the sprite for a state (C# Define), preconfigured; it becomes active when it is the current state. */
  Define(state: number, source: string, columns: number, rows: number, fps = 15, repeat = -1, autoPlay = true, placement?: SpritePlacementConfig): this {
    const s = new SkiaSprite();
    s.UseCache = "Image";
    s.HorizontalOptions = "Fill"; s.VerticalOptions = "Fill";
    s.AutoPlay = autoPlay; s.Repeat = repeat; s.FramesPerSecond = fps; s.Columns = columns; s.Rows = rows;
    s.Source = source;
    s.ApplyPlacementConfig(placement);
    this.sprites.set(state, s);
    if (!this.active && state === this.state) this.SetActive(s);
    return this;
  }

  /** The hit box follows the drawn frame of the active sprite (C# HitBoxAuto). */
  override get HitBoxAuto(): SKRect {
    const d = this.active?.DisplayRect;
    return d && d.Width > 0 ? d : super.HitBoxAuto;
  }

  private SetActive(sprite: SkiaSprite): void {
    if (this.active === sprite) return;
    if (this.active) { this.RemoveSubView(this.active); this.active.Stop(); }
    this.active = sprite;
    this.AddSubView(sprite);
    if (sprite.TotalFrames > 0) sprite.Start();
  }

  /** Base: swap the active child to the pre-created sprite for newState. */
  protected OnChangeState(_oldState: number, newState: number): void {
    const sprite = this.sprites.get(newState);
    if (sprite) this.SetActive(sprite);
  }

  protected override DisposeChildren(): void {
    super.DisposeChildren();
    for (const s of this.sprites.values()) s.Dispose();
    this.sprites.clear();
    this.active = undefined;
  }
}
