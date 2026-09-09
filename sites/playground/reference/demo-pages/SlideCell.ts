import { Colors, SkiaDynamicDrawnCell, SkiaLabel, SkiaShape, Thickness } from "drawnui-react/core";

export interface Slide { title: string; color: string; height?: number }

/** Recycled carousel slide: visuals built once, SetContent runs on every rebind (like ContactCell). */
export class SlideCell extends SkiaDynamicDrawnCell {
  private readonly shape = new SkiaShape();
  private readonly label = new SkiaLabel();
  private readonly sub = new SkiaLabel();

  constructor() {
    super();
    this.shape.Type = "Rectangle";
    this.shape.CornerRadius = 12;
    this.shape.HorizontalOptions = "Fill";
    this.shape.VerticalOptions = "Fill";
    this.label.FontSize = 22;
    this.label.FontFamily = "FontTextBold";
    this.label.TextColor = Colors.White;
    this.label.HorizontalOptions = "Center";
    this.label.VerticalOptions = "Center";
    this.sub.FontSize = 12;
    this.sub.TextColor = "#FFFFFFAA";
    this.sub.HorizontalOptions = "Center";
    this.sub.VerticalOptions = "End";
    this.sub.Margin = new Thickness(0, 0, 0, 10);
    this.shape.AddSubView(this.label);
    this.shape.AddSubView(this.sub);
    this.AddSubView(this.shape);
  }

  protected override SetContent(item: unknown): void {
    const s = item as Slide;
    this.shape.BackgroundColor = s.color;
    this.label.Text = s.title;
    this.sub.Text = `recycled cell · index ${this.ContextIndex}`;
    if (s.height) this.HeightRequest = s.height;
  }
}
