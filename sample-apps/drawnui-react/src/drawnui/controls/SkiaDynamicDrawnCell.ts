import { SkiaLayout } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SkiaDynamicDrawnCell: base for recycled cells. Build the visuals once in the
 * constructor, override SetContent(ctx) to push the bound item into them — it runs on every rebind.
 */
export class SkiaDynamicDrawnCell extends SkiaLayout {
  constructor() {
    super();
    this.HorizontalOptions = "Fill";
  }

  protected override OnBindingContextChanged(): void {
    super.OnBindingContextChanged();
    this.SetContent(this.BindingContext);
  }

  /** Called with the item (BindingContext) whenever the cell is (re)bound. */
  protected SetContent(_ctx: unknown): void {}
}
