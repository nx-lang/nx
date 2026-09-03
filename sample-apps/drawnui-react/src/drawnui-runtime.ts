/**
 * DrawnUI startup, shared by every view in the app.
 *
 * Mirrors the vendored demo's own bootstrap — `Super.UseDrawnUi().ConfigureFonts(...).BuildAsync()`
 * — so text in the fiddle measures the same as text in the DrawnUI original it was ported from.
 */
import { Aria } from "./drawnui/react/index";
import { SkiaButton, SkiaLabel, Super } from "./drawnui/index";

export const drawnUiReady: Promise<void> = Super.UseDrawnUi()
  .ConfigureFonts((fonts) =>
    fonts
      .AddFont("/fonts/OpenSans-Regular.ttf", "FontText")
      .AddFont("/fonts/OpenSans-Semibold.ttf", "FontText", 600)
      .AddFont("/fonts/OpenSans-Semibold.ttf", "FontTextBold")
      .AddSymbols()
      .AddEmojis(),
  )
  .BuildAsync()
  .then(() => {
    SkiaLabel.DefaultAccessibilityRole = Aria.RoleText;
    SkiaButton.DefaultAccessibilityRole = Aria.RoleButton;
  });
