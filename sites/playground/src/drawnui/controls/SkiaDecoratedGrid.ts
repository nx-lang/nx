import type { DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { SKRect, type SkiaGradient } from "../core/Types";
import { SkiaGrid } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SkiaDecoratedGrid: a SkiaGrid painting separator lines in the spacing between its tracks,
 * `VerticalLine` (a ColumnSpacing wide gradient between columns) and `HorizontalLine` (a RowSpacing tall gradient
 * over black between rows), set either to undefined to skip. Lines draw under the children (C# ZIndex -1 overlay).
 */
export class SkiaDecoratedGrid extends SkiaGrid {
  static readonly HorizontalGradient: SkiaGradient = { Type: "Linear", Colors: ["#00E8E3D7", "#78E8E3D7", "#78E8E3D7", "#00E8E3D7"], ColorPositions: [0, 0.1, 0.9, 1], StartXRatio: 0, StartYRatio: 0, EndXRatio: 1, EndYRatio: 0 };
  static readonly VerticalGradient: SkiaGradient = { Type: "Linear", Colors: ["#00E8E3D7", "#78E8E3D7", "#78E8E3D7", "#00E8E3D7"], ColorPositions: [0, 0.1, 0.9, 1], StartXRatio: 0, StartYRatio: 0, EndXRatio: 0, EndYRatio: 1 };

  HorizontalLine?: SkiaGradient = SkiaDecoratedGrid.HorizontalGradient;
  VerticalLine?: SkiaGradient = SkiaDecoratedGrid.VerticalGradient;

  protected override Paint(ctx: DrawingContext): void {
    const g = this.GridStructure;
    if (g) {
      const CK = Super.CK, canvas = ctx.Context.Canvas, scale = ctx.Scale, p = this.Padding, r = this.DrawingRect;
      const inner = new SKRect(r.Left + p.Left * scale, r.Top + p.Top * scale, r.Right - p.Right * scale, r.Bottom - p.Bottom * scale);
      const paint = new CK.Paint();
      paint.setAntiAlias(true);
      if (this.VerticalLine && this.ColumnSpacing > 0) {
        for (let col = 1; col < g.Columns.length; col++) {
          const x = inner.Left + (g.LeftEdgeOfColumn(col) - this.ColumnSpacing) * scale;
          const rect = new SKRect(x, inner.Top, x + this.ColumnSpacing * scale, inner.Bottom);
          paint.setShader(null);
          if (this.SetupGradient(paint, this.VerticalLine, rect)) canvas.drawRect(CK.LTRBRect(rect.Left, rect.Top, rect.Right, rect.Bottom), paint);
        }
      }
      if (this.HorizontalLine && this.RowSpacing > 0) {
        for (let row = 1; row < g.Rows.length; row++) {
          const y = inner.Top + (g.TopEdgeOfRow(row) - this.RowSpacing) * scale;
          const rect = new SKRect(inner.Left, y, inner.Right, y + this.RowSpacing * scale);
          const skRect = CK.LTRBRect(rect.Left, rect.Top, rect.Right, rect.Bottom);
          paint.setShader(null); paint.setColor(CK.BLACK); // C#: a black SkiaShape under the gradient
          canvas.drawRect(skRect, paint);
          if (this.SetupGradient(paint, this.HorizontalLine, rect)) canvas.drawRect(skRect, paint);
        }
      }
      paint.delete();
    }
    super.Paint(ctx);
  }
}
