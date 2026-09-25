import { ScaledSize, SkiaLayout, type DrawingContext } from "drawnui-react/core";

/**
 * Fits a fixed logical viewport (LogicalWidth x LogicalHeight points) into the available space by changing the
 * RENDERING SCALE of its children, not by a Scale transform: children are measured, arranged and drawn at the
 * fitted scale, so layout, hit-testing and gestures just work. The layout-level twin of Pong.Shared's
 * RescalingCanvas (DrawnUi Blazor AspectLockedCanvas, ArtOfFoto RescalingLayout).
 */
export class RescalingLayout extends SkiaLayout {
  LogicalWidth = 0;
  LogicalHeight = 0;
  /** The fitted scale the children use. */
  ContextScale = 1;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.VerticalOptions = "Fill";
  }

  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    if (this.LogicalWidth <= 0 || this.LogicalHeight <= 0 || !isFinite(widthConstraint) || !isFinite(heightConstraint)) {
      this.ContextScale = scale;
      return super.MeasureAbsolute(widthConstraint, heightConstraint, scale);
    }
    this.ContextScale = scale * Math.min(widthConstraint / (this.LogicalWidth * scale), heightConstraint / (this.LogicalHeight * scale));
    super.MeasureAbsolute(widthConstraint, heightConstraint, this.ContextScale);
    return ScaledSize.FromPixels(widthConstraint, heightConstraint, scale);
  }

  protected override OnLayoutChanged(): void {
    // children are arranged at the fitted scale
    const own = this.RenderingScale;
    this.RenderingScale = this.ContextScale;
    try { super.OnLayoutChanged(); } finally { this.RenderingScale = own; }
  }

  protected override Paint(ctx: DrawingContext): void {
    super.Paint({ ...ctx, Scale: this.ContextScale });
  }
}
