/**
 * Writes `catalog.nx`, the catalog of the `large-catalog` conformance program: a host's library of
 * external components, of the size and shape of a real one. The counts are those measured on the
 * catalog the DrawnUI fiddle generates from its component package: 45 external components of which
 * 11 are abstract bases, 37 types (31 enumerations and 6 records), about 380 properties, and
 * inheritance three levels deep. The names are this script's own; nothing is copied from a host.
 *
 *   node specs/ir-conformance/large-catalog/generate-catalog.mjs
 *
 * It takes no input and reads no clock, so it writes the same bytes every time. Run it after
 * changing the tables below or when the language's syntax for a declaration changes, then
 * regenerate the corpus's expected files (`specs/ir-conformance/README.md`).
 */
import { writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

const enumerations = {
  Align: "Start Center End Fill",
  Axis: "Horizontal Vertical",
  BarVisibility: "Auto Always Never",
  Blend:
    "Clear Source Destination SourceOver DestinationOver SourceIn DestinationIn SourceOut DestinationOut " +
    "SourceAtop DestinationAtop Xor Plus Modulate Screen Overlay Darken Lighten ColorDodge ColorBurn " +
    "HardLight SoftLight Difference Exclusion Multiply Hue Saturation Color Luminosity",
  CacheKind: "None Image Operations Memory Gpu",
  ClipKind: "None Bounds Outline",
  ColorFilter: "None Grayscale Sepia Invert Tint",
  DrawerEdge: "Left Top Right Bottom",
  Easing: "Linear EaseIn EaseOut EaseInOut Spring",
  FontStyle: "Upright Italic Oblique",
  FontWeight: "Thin Light Regular Medium SemiBold Bold Black",
  GradientKind: "None Linear Radial Sweep",
  ImageFit: "Fill Contain Cover ScaleDown None",
  ImageQuality: "Low Medium High",
  IndicatorKind: "Ring Bar Dots",
  KeyboardKind: "Text Number Email Phone Url",
  LayoutKind: "Vertical Horizontal Layered Flow Table",
  PointerMode: "Auto PassThrough Block",
  ReturnKey: "Done Next Find Send Go",
  ScrollMode: "None Vertical Horizontal Both",
  ShapeKind: "Rectangle Circle Ellipse Arc Outline Polygon Segment",
  SliderKind: "Single Range",
  SnapMode: "None Start Center End",
  StrokeCap: "Butt Round Square",
  StrokeJoin: "Miter Round Bevel",
  TextAlign: "Start Center End Justify",
  TextCase: "AsWritten Lower Upper Title",
  TextTrim: "None Character Word Ellipsis",
  TouchEffect: "None Ripple Highlight Shrink",
  TransitionKind: "None Fade Slide Zoom Flip",
  Visibility: "Visible Hidden Collapsed",
};

/** A record's fields, each `Name:type = default`. */
const records = {
  Corners: ["TopLeft:float64 = 0", "TopRight:float64 = 0", "BottomRight:float64 = 0", "BottomLeft:float64 = 0"],
  Edge: ["Color:string = \"#000000\"", "Width:float64 = 1", "Dash:string = \"\""],
  Gradient: ["Kind:GradientKind = {GradientKind.Linear}", "Angle:float64 = 0", "Colors:string+", "Stops?:float64+"],
  Insets: ["Left:float64 = 0", "Top:float64 = 0", "Right:float64 = 0", "Bottom:float64 = 0"],
  Point: ["X:float64 = 0", "Y:float64 = 0"],
  Shadow: ["Color:string = \"#000000\"", "X:float64 = 0", "Y:float64 = 0", "Blur:float64 = 0"],
};

/**
 * The components in declaration order. `props` maps a type to the names of the optional
 * properties of that type; `defaults` are properties with a default, written out; `content` is
 * the content property; `emits` the actions, each with its fields.
 */
const components = [
  { name: "Node", abstract: true },
  {
    name: "View",
    base: "Node",
    abstract: true,
    props: {
      string: "Tag StyleClass BackgroundColor TintColor AutomationId AutomationLabel TransitionName",
      float64:
        "Width Height MinWidth MinHeight MaxWidth MaxHeight Left Top Opacity Rotation Scale ScaleX ScaleY SkewX " +
        "SkewY TranslateX TranslateY AnchorX AnchorY ZIndex AspectRatio HorizontalFill VerticalFill Elevation " +
        "HitSlop FadeSeconds",
      boolean: "IsHidden ClipsToBounds IgnoresInput IsGhost UsesLayer AnimatesLayout BlocksGestures IsSelected",
      Align: "HorizontalAlign VerticalAlign",
      Visibility: "Visibility",
      CacheKind: "Cache",
      Blend: "Blend",
      ClipKind: "Clip",
      PointerMode: "Pointer",
      TouchEffect: "Touch",
      Insets: "Margin Padding",
      Corners: "Corners",
      Shadow: "Shadow",
      Gradient: "Background",
      Edge: "Border",
    },
    defaults: ["IsEnabled:boolean = true"],
    emits: ["Tapped { }", "LongPressed { }", "Appeared { }", "Disappeared { }"],
  },
  {
    name: "LayoutBase",
    base: "View",
    abstract: true,
    props: {
      string: "Columns Rows ItemKeyPath EmptyText GroupKeyPath HeaderText FooterText",
      float64:
        "RowSpacing ColumnSpacing ItemWidth ItemHeight EstimatedItemSize InflateBefore InflateAfter SplitRatio " +
        "RecycleMargin LoadMoreThreshold StickyOffset",
      int: "MaxColumns MaxRows BatchSize InitialIndex",
      boolean:
        "IsVirtualized RecyclesCells WrapsContent EqualizesHeights EqualizesWidths IsReversed FillsLastRow " +
        "KeepsPosition ShowsEmptyView IsGrouped HasStickyHeaders",
      LayoutKind: "Kind",
      Axis: "Direction",
      Align: "JustifyContent AlignItems",
      Insets: "ItemPadding",
      Corners: "ItemCorners",
    },
    defaults: ["Spacing:float64 = 0"],
    emits: ["ItemAppeared { index:int }", "ItemDisappeared { index:int }", "SelectionChanged { index:int }"],
  },
  {
    name: "Layout",
    base: "LayoutBase",
    props: { float64: "CacheScale", boolean: "KeepsCacheWhenHidden RedrawsOnResize IsMirrored" },
    content: "Children?:Node+",
  },
  {
    name: "Stack",
    base: "LayoutBase",
    props: { boolean: "StretchesItems", int: "MaxItemsShown" },
    content: "Children?:Node+",
  },
  {
    name: "Row",
    base: "LayoutBase",
    props: { boolean: "AlignsBaselines WrapsWhenNarrow", float64: "MinItemWidth" },
    content: "Children?:Node+",
  },
  {
    name: "Column",
    base: "LayoutBase",
    props: { boolean: "DistributesEvenly", float64: "MinItemHeight" },
    content: "Children?:Node+",
  },
  {
    name: "Wrap",
    base: "LayoutBase",
    props: { float64: "LineSpacing", int: "MaxLinesShown", Align: "LineAlign", boolean: "TrimsTrailingSpacing" },
    content: "Children?:Node+",
  },
  {
    name: "Grid",
    base: "LayoutBase",
    props: {
      string: "DefaultColumn DefaultRow LineColor",
      int: "ColumnCount RowCount",
      float64: "LineWidth",
      boolean: "ShowsLines",
    },
    content: "Children?:Node+",
  },
  {
    name: "TextBase",
    base: "View",
    abstract: true,
    props: {
      string: "FontFamily TextColor StrokeColor ShadowColor LinkColor Locale FallbackFamily",
      float64:
        "LineHeight LineSpacing LetterSpacing ParagraphSpacing StrokeWidth MaxFontSize MinFontSize AutoSizeStep " +
        "DropShadowSize FirstLineIndent",
      int: "MaxLines MinLines",
      boolean: "AutoSize IsSelectable IsUnderlined IsStruckThrough KeepsSpacesOnLineBreak UsesFontScaling",
      FontWeight: "Weight",
      FontStyle: "Style",
      TextAlign: "HorizontalTextAlign",
      Align: "VerticalTextAlign",
      TextCase: "Case",
      TextTrim: "Trim",
      Gradient: "TextGradient",
      Shadow: "TextShadow",
    },
    defaults: ["FontSize:float64 = 14"],
  },
  {
    name: "Label",
    base: "TextBase",
    props: { string: "Text FormatString", int: "HeadingLevel", boolean: "IsHeading IsMonospaced" },
  },
  {
    name: "RichText",
    base: "TextBase",
    props: { string: "Markup LinkStyleClass CodeFontFamily", boolean: "AllowsLinks OpensLinksExternally" },
  },
  {
    name: "Link",
    base: "TextBase",
    props: { string: "Text Url VisitedColor", boolean: "OpensExternally IsVisited" },
    emits: ["Opened { url:string }"],
  },
  {
    name: "ShapeBase",
    base: "View",
    abstract: true,
    props: {
      string: "FillColor StrokeColor Data DashPattern",
      float64:
        "StrokeWidth StartAngle SweepAngle InnerRadius SmoothFactor StrokeMiter DashPhase CornerSmoothing " +
        "RotationX RotationY Perspective CameraDistance",
      boolean: "IsClosed IsAntialiased FillsWithStroke",
      ShapeKind: "Kind",
      StrokeCap: "Cap",
      StrokeJoin: "Join",
      Gradient: "Fill StrokeGradient",
      Shadow: "InnerShadow",
      Point: "Pivot",
      "Point+": "Points",
    },
  },
  { name: "Shape", base: "ShapeBase", content: "Children?:Node+" },
  {
    name: "Curve",
    base: "ShapeBase",
    props: { float64: "ViewportWidth ViewportHeight", boolean: "ScalesToFit", Align: "HorizontalContentAlign" },
  },
  { name: "Line", base: "ShapeBase", props: { float64: "X1 Y1 X2 Y2", string: "StartMarker EndMarker" } },
  {
    name: "Frame",
    base: "ShapeBase",
    props: { boolean: "HasShadow", string: "Title", float64: "TitleSize" },
    content: "Children?:Node+",
  },
  {
    name: "ImageBase",
    base: "View",
    abstract: true,
    props: {
      string: "Source Placeholder ErrorSource OverlayColor",
      float64: "ZoomX ZoomY OffsetX OffsetY Blur LoadDelaySeconds Brightness Contrast Saturation ParallaxX ParallaxY",
      boolean: "PreloadsSource UsesCacheFile IsAnimated ErasesOnChange LoadsOnAppear",
      ImageFit: "Fit",
      ImageQuality: "Quality",
      ColorFilter: "Filter",
      Align: "HorizontalImageAlign VerticalImageAlign",
      Insets: "Crop",
    },
    emits: ["Loaded { }", "Failed { message:string }"],
  },
  {
    name: "Image",
    base: "ImageBase",
    props: { string: "AltText CacheKey", float64: "FadeInSeconds", boolean: "IsDecorative" },
  },
  { name: "Icon", base: "ImageBase", props: { string: "Name FontFamily", float64: "Size", boolean: "IsFilled" } },
  { name: "Svg", base: "ImageBase", props: { string: "Markup", boolean: "UsesIntrinsicSize" } },
  {
    name: "Animation",
    base: "ImageBase",
    props: { boolean: "IsPlaying AutoPlays Reverses", int: "Repeat StartFrame EndFrame", float64: "Speed" },
    emits: ["Finished { }"],
  },
  {
    name: "Avatar",
    base: "ImageBase",
    props: { string: "Initials StatusColor", boolean: "ShowsStatus", Edge: "Ring" },
  },
  {
    name: "InputBase",
    base: "View",
    abstract: true,
    props: {
      string: "Value Placeholder PlaceholderColor CursorColor SelectionColor TextColor FontFamily FocusGroup Tooltip",
      int: "MaxLength CursorPosition",
      float64: "FontSize",
      boolean:
        "IsReadOnly IsPassword IsSpellChecked IsAutoCorrected ClearsOnFocus SelectsAllOnFocus IsFocusable " +
        "AutoCapitalizes",
      KeyboardKind: "Keyboard",
      ReturnKey: "Return",
      TextAlign: "TextAlign",
    },
    emits: ["Changed { value:string }", "Submitted { value:string }", "Focused { }", "Blurred { }"],
  },
  { name: "Entry", base: "InputBase", props: { boolean: "ShowsClearButton", string: "Prefix Suffix" } },
  {
    name: "Editor",
    base: "InputBase",
    props: { int: "MinLinesShown MaxLinesShown", boolean: "AutoGrows ShowsCounter" },
  },
  {
    name: "Search",
    base: "InputBase",
    props: { string: "CancelText IconName", float64: "DebounceSeconds", boolean: "ShowsCancel" },
  },
  {
    name: "RangeBase",
    base: "View",
    abstract: true,
    props: {
      string: "TrackColor ActiveColor ThumbColor",
      float64: "Value Step TrackHeight ThumbSize",
      boolean: "IsContinuous ShowsValue",
    },
    defaults: ["Minimum:float64 = 0", "Maximum:float64 = 1"],
    emits: ["ValueChanged { value:float64 }"],
  },
  {
    name: "Slider",
    base: "RangeBase",
    props: { SliderKind: "Kind", float64: "UpperValue", int: "TickCount", boolean: "ShowsTicks" },
  },
  {
    name: "Progress",
    base: "RangeBase",
    props: { IndicatorKind: "Indicator", string: "Caption", boolean: "IsIndeterminate IsRounded" },
  },
  { name: "Stepper", base: "RangeBase", props: { boolean: "WrapsAround" } },
  {
    name: "ScrollBase",
    base: "View",
    abstract: true,
    props: {
      float64: "ScrollX ScrollY Friction OverscrollDistance SnapInterval RefreshThreshold",
      boolean: "IsScrollEnabled Bounces LocksDirection IsRefreshing ZoomsContent",
      ScrollMode: "Mode",
      SnapMode: "Snap",
      BarVisibility: "Bars",
    },
    emits: ["Scrolled { x:float64 y:float64 }", "Refreshed { }"],
  },
  {
    name: "Scroll",
    base: "ScrollBase",
    props: { float64: "ContentWidth ContentHeight" },
    content: "Children?:Node+",
  },
  {
    name: "List",
    base: "ScrollBase",
    props: {
      float64: "ItemSpacing",
      string: "HeaderText FooterText SeparatorColor",
      int: "InitialItem",
      boolean: "ShowsSeparators",
    },
    content: "Children?:Node+",
  },
  {
    name: "Carousel",
    base: "ScrollBase",
    props: { int: "SelectedIndex", float64: "PeekAmount AutoAdvanceSeconds", boolean: "IsLooping ShowsIndicators" },
    content: "Children?:Node+",
  },
  {
    name: "OverlayBase",
    base: "View",
    abstract: true,
    props: {
      string: "ScrimColor",
      float64: "DimAmount OpenSeconds",
      boolean: "IsOpen ClosesOnOutsideTap IsModal AnimatesOpen IgnoresSafeArea",
      TransitionKind: "Transition Enter Exit",
      Easing: "Easing",
      Point: "Offset",
    },
    emits: ["Opened { }", "Closed { }"],
  },
  {
    name: "Drawer",
    base: "OverlayBase",
    props: { DrawerEdge: "Edge", float64: "HeaderSize PeekSize", boolean: "IsSwipeEnabled" },
    content: "Children?:Node+",
  },
  {
    name: "Popup",
    base: "OverlayBase",
    props: { Point: "Anchor", float64: "ArrowSize", Align: "Placement", boolean: "HasArrow" },
    content: "Children?:Node+",
  },
  {
    name: "Sheet",
    base: "OverlayBase",
    props: { "float64+": "Detents", int: "InitialDetent", boolean: "ShowsGrabber IsDismissable" },
    content: "Children?:Node+",
  },
  {
    name: "ToggleBase",
    base: "View",
    abstract: true,
    props: { string: "OnColor OffColor", boolean: "AnimatesChange" },
    defaults: ["IsOn:boolean = false"],
    emits: ["Toggled { isOn:boolean }"],
  },
  { name: "Switch", base: "ToggleBase", props: { string: "ThumbColor" } },
  { name: "Checkbox", base: "ToggleBase", props: { string: "CheckColor Caption", float64: "BoxSize" } },
  {
    name: "Button",
    base: "View",
    props: {
      string: "Text IconName TextColor FontFamily TouchColor Tooltip",
      float64: "FontSize IconSize LongPressSeconds",
      boolean: "IsBusy IsPrimary",
      FontWeight: "Weight",
    },
    content: "Children?:Node+",
  },
  {
    name: "Spinner",
    base: "View",
    props: { string: "Color", float64: "Size StrokeWidth", IndicatorKind: "Indicator" },
  },
];

const words = (text) => text.split(" ").filter((word) => word !== "");

/** A component's own properties as lines, in the order of its tables, and how many there are. */
function ownProperties(component) {
  const lines = [];
  for (const [type, names] of Object.entries(component.props ?? {})) {
    for (const name of words(names)) {
      lines.push(`${name}?:${type}`);
    }
  }
  lines.push(...(component.defaults ?? []));
  if (component.content !== undefined) {
    lines.push(`content ${component.content}`);
  }
  return lines;
}

const propertyName = (line) => line.replace(/^content /, "").match(/^[A-Za-z0-9]+/)[0];

// A property is declared once along a chain of bases, and every type a table names is declared.
const byName = new Map(components.map((component) => [component.name, component]));
const primitives = new Set(["string", "float64", "int", "boolean"]);
for (const component of components) {
  const seen = new Map();
  for (let current = component; current !== undefined; current = byName.get(current.base)) {
    for (const line of ownProperties(current)) {
      const name = propertyName(line);
      if (seen.has(name)) {
        throw new Error(`${component.name}: ${name} is declared by ${seen.get(name)} and by ${current.name}`);
      }
      seen.set(name, current.name);
    }
  }
  for (const type of Object.keys(component.props ?? {})) {
    const named = type.replace(/\+$/, "");
    if (!primitives.has(named) && enumerations[named] === undefined && records[named] === undefined) {
      throw new Error(`${component.name}: no type ${named}`);
    }
  }
}

const lines = [
  "// Generated by generate-catalog.mjs. Do not edit by hand: change the script's tables and run it.",
  "//",
  "// A host's library of external components, of the size and shape of a real one: the host prepares",
  "// this image once and links every snippet against it.",
  "",
];
for (const [name, cases] of Object.entries(enumerations)) {
  const oneLine = `export type ${name} = ${words(cases).join(" | ")}`;
  if (oneLine.length <= 100) {
    lines.push(oneLine, "");
  } else {
    lines.push(`export type ${name} =`, ...words(cases).map((item) => `  | ${item}`), "");
  }
}
for (const [name, fields] of Object.entries(records)) {
  lines.push(`export type ${name} = {`, ...fields.map((field) => `  ${field}`), "}", "");
}
let properties = 0;
for (const component of components) {
  const head = `export ${component.abstract ? "abstract " : ""}external component`;
  const open = `<${component.name}${component.base === undefined ? "" : ` extends ${component.base}`}`;
  const own = ownProperties(component);
  properties += own.length;
  const emits = component.emits ?? [];
  if (own.length === 0 && emits.length === 0) {
    lines.push(`${head} ${open} />`, "");
    continue;
  }
  lines.push(head, open, ...own.map((line) => `  ${line}`));
  if (emits.length > 0) {
    lines.push("  emits {", ...emits.map((action) => `    ${action}`), "  }");
  }
  lines.push("/>", "");
}
lines.pop();

/** The text of `catalog.nx`. The TypeScript runtime's tests check the committed file against it. */
export const catalogText = `${lines.join("\n")}\n`;

// Run as a script, it writes the file; loaded as a module, it only offers the text.
if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  writeFileSync(fileURLToPath(new URL("catalog.nx", import.meta.url)), catalogText);
  const abstract = components.filter((component) => component.abstract).length;
  const types = Object.keys(enumerations).length + Object.keys(records).length;
  console.log(`catalog.nx: ${components.length} components (${abstract} abstract), ${types} types, ${properties} properties`);
}
