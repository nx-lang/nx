import { Colors, SkiaDynamicDrawnCell, SkiaLabel, SkiaShape, SkiaStack, Thickness } from "drawnui-react/core";

export interface FeedItem { Id: number; Title: string; Body: string; Color: string }

/** Uneven recycled cell: body text wraps to 1..6 lines, so every row has its own height (MeasureVisible territory). */
export class FeedCell extends SkiaDynamicDrawnCell {
  private readonly stripe = new SkiaShape();
  private readonly title = new SkiaLabel();
  private readonly body = new SkiaLabel();
  private readonly footer = new SkiaLabel();

  constructor() {
    super();
    this.Type = "Absolute";
    this.Padding = new Thickness(16, 12, 16, 12);
    this.BackgroundColor = "#111827";
    this.UseCache = "ImageDoubleBuffered"; // previous bitmap shown while a cell is re-recorded; DrawPlaceholder when none

    this.stripe.Type = "Rectangle";
    this.stripe.CornerRadius = 3;
    this.stripe.WidthRequest = 6;
    this.stripe.VerticalOptions = "Fill";

    const column = new SkiaStack();
    column.Spacing = 6;
    column.Margin = new Thickness(18, 0, 0, 0);
    this.title.FontSize = 15;
    this.title.FontFamily = "FontTextBold";
    this.title.TextColor = Colors.White;
    this.body.FontSize = 13;
    this.body.TextColor = "#CBD5E1";
    this.body.HorizontalOptions = "Fill";
    this.footer.FontSize = 11;
    this.footer.TextColor = "#64748B";
    column.AddSubView(this.title);
    column.AddSubView(this.body);
    column.AddSubView(this.footer);

    this.AddSubView(this.stripe);
    this.AddSubView(column);
  }

  protected override SetContent(ctx: unknown): void {
    const item = ctx as FeedItem;
    this.stripe.BackgroundColor = item.Color;
    this.title.Text = item.Title;
    this.body.Text = item.Body;
    this.footer.Text = `#${item.Id} · ${item.Body.length} chars`;
  }
}
