/**
 * DrawnUI startup, shared by every view in the app.
 *
 * Mirrors the vendored demo's own bootstrap — `Super.UseDrawnUi().ConfigureFonts(...)
 * .ConfigureStyles(...).BuildAsync()` — so text in the playground measures the same as text in the
 * DrawnUI original it was ported from. The fonts are named from the site's prefix rather than the
 * origin's root, since the site does not own the root.
 *
 * The styles matter: DrawnUI resolves an empty `FontFamily` to Skia's built-in face, so without the
 * defaults below every label that names no family — most of them — would draw in a different
 * typeface than the demo. A button pushes its own `FontFamily` onto its caption, so the label style
 * never reaches it; the button gets its own entry.
 */
import { Aria } from "./drawnui/react/index";
import { SkiaButton, SkiaLabel, Super } from "./drawnui/index";
import { BASE_URL } from "./paths";

export const drawnUiReady: Promise<void> = Super.UseDrawnUi()
  .ConfigureFonts((fonts) =>
    fonts
      .AddFont(`${BASE_URL}fonts/OpenSans-Regular.ttf`, "FontText")
      .AddFont(`${BASE_URL}fonts/OpenSans-Semibold.ttf`, "FontText", 600)
      .AddFont(`${BASE_URL}fonts/OpenSans-Semibold.ttf`, "FontTextBold")
      .AddSymbols()
      .AddEmojis(),
  )
  .ConfigureStyles((styles) =>
    styles
      .AddStyle({
        TargetType: SkiaLabel,
        ApplyToDerivedTypes: true,
        Setters: { FontFamily: "FontText" },
      })
      .AddStyle({
        TargetType: SkiaButton,
        ApplyToDerivedTypes: true,
        Setters: { FontFamily: "FontText" },
      }),
  )
  .BuildAsync()
  .then(() => {
    SkiaLabel.DefaultAccessibilityRole = Aria.RoleText;
    SkiaButton.DefaultAccessibilityRole = Aria.RoleButton;
  });
