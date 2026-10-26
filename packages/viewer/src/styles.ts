/**
 * The element's stylesheet. Colors follow the page's `color-scheme` through `light-dark()`, and a
 * host can override each through the custom properties named here.
 */
export const styles = `
:host {
  display: block;
  position: relative;
  font-family: var(--nx-viewer-font-family, system-ui, -apple-system, "Segoe UI", sans-serif);
  font-size: var(--nx-viewer-font-size, 14px);
  line-height: 1.5;
  color: var(--nx-viewer-foreground, CanvasText);
  background: var(--nx-viewer-background, transparent);
  --blue-soft: light-dark(#e5ecff, #1f2a4d);
  --gold: light-dark(#8a5a00, #ffb020);
  --gold-soft: light-dark(#fff1d1, #3a2d12);
  --mono: var(--nx-viewer-mono-font-family, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
}
:host([hidden]) { display: none; }
* { box-sizing: border-box; }
.frame { position: relative; }
/* The toolbar takes no room: what it shows floats over the reading's top right corner, so showing it
   moves nothing. */
.toolbar {
  position: sticky;
  top: 0;
  z-index: 2;
  height: 0;
}
.toolbar:empty { display: none; }
.badge {
  position: absolute;
  top: 4px;
  right: 8px;
  font-size: 11px;
  padding: 0 6px;
  border-radius: 4px;
  background: var(--gold-soft);
  color: var(--gold);
}
.announcer { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
.body:focus-visible { outline: 2px solid var(--blue-soft); outline-offset: 4px; border-radius: 4px; }
.body.by-pointer:focus-visible { outline: none; }

/* The reading: the document's text as an editor shows it, with elements read as groups and lines.
   A node is an inline span, so marking one changes no line's height. */
.source {
  margin: 0;
  /* Text stops 4px inside where a selected or hovered row's highlight ends, as it starts 4px inside
     where the highlight begins. */
  /* The left padding is room for a declaration's fold chevron, which hangs left of its first line. */
  padding: 4px 12px 12px 1.4em;
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-variation-settings: var(--nx-viewer-source-font-variation, normal);
  font-feature-settings: var(--nx-viewer-source-font-features, normal);
  font-size: var(--nx-viewer-source-font-size, 13px);
  line-height: var(--nx-viewer-source-line-height, 18px);
  white-space: pre;
  tab-size: 2;
}
/* A tree out of date for a moment, as while an edit's tree is on its way, reads as it is: the muting
   and the badge come only once it stays out of date, and go at once. */
:host([stale]) .source { opacity: 0.55; transition: opacity 0.15s ease-out 0.5s; }
.badge:not([hidden]) { animation: nx-viewer-late 0.15s ease-out 0.5s both; }
@keyframes nx-viewer-late { from { opacity: 0; } }
.source .t { color: light-dark(var(--l, inherit), var(--d, inherit)); }
.source .n { border-radius: 2px; }
/* Text shown to people, as a \`uitext\` property holds: its words in the prose font and a plain,
   clear color of their own, since they are content and every hue in a theme means code; its quotes
   quiet. The words sit in a box no wider than the room left on their line, so a long string wraps
   and a string of several lines continues under its first. */
.source .uitext-body {
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, inherit);
  color: var(--nx-viewer-uitext-color, light-dark(#222222, #e4e4e4));
}
/* The quotes are in the words' font, so they sit against the words and not a character away. */
.source .uitext-quote {
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, inherit);
  color: var(--nx-viewer-uitext-quote-color, light-dark(#8e8e8e, #808080));
}
.source .uitext-block {
  display: inline-flex;
  vertical-align: baseline;
  max-width: max(20ch, calc(100% - var(--column, 0) * 1ch));
}
.source .uitext-block > .uitext-body {
  min-width: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  line-height: var(--nx-viewer-source-line-height, 18px);
}
.source .n.selected > .uitext-block { background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
/* A string its visualizer formats as an identifier: one quiet token, as products that show ids do,
   in muted gray on a faint tint, with no border and no added height. */
.source .id-value {
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  color: var(--nx-viewer-id-color, light-dark(#8c959f, #858585));
  background: var(--nx-viewer-id-background, light-dark(#0000000a, #ffffff0f));
  border-radius: 6px;
  padding: 0 6px;
}
/* A chip last in its tag closes it with a small gap, less than a space, before the \`/>\`. */
.source .id-end { margin-right: 3px; }
.source .id-quote { color: var(--nx-viewer-id-quote-color, light-dark(#afb8c1, #6e6e6e)); }
/* An element as a property sheet: its name, then a row to an attribute, its names in one quiet
   column and its values in another, and what it holds under them. It sits where the element began
   on its line and is no wider than the room left there. */
.source .sheet {
  display: inline-grid;
  grid-template-columns: max-content minmax(0, 1fr);
  column-gap: 2ch;
  vertical-align: top;
  max-width: calc(100% - var(--indent, 0) * 1ch);
}
/* A sheet inside another is already set in by it, so only its container bounds it. */
.source .sheet .sheet { max-width: 100%; }
.source .sheet-head, .source .sheet-held, .source .sheet-note { grid-column: 1 / -1; }
.source .sheet-row { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; padding-left: 2ch; }
.source .sheet-held, .source .sheet-note { padding-left: 2ch; }
.source .sheet-name { opacity: 0.75; }
.source .sheet-value { min-width: 0; white-space: pre-wrap; overflow-wrap: break-word; }
/* Inside a sheet, UI text wraps where its line does: the sheet already sets it in from the margin. */
.source .sheet .uitext-block { display: inline; max-width: none; }
.source .sheet-list, .source .sheet-held { display: block; }
.source .sheet-item { display: block; }
/* An element on one line: its name, then its properties, a name in the same quiet tone as a
   sheet's and its value after it. */
.source .line-name { opacity: 0.75; }
/* A one-line element's \`<\` and \`/>\`, quiet like its \`=\`. The \`>\` is drawn in close under the slash,
   nearly touching it, so the two read as one mark. */
.source .line-open, .source .line-close { color: var(--nx-viewer-tag-color, light-dark(#6e6e6e, #9a9a9a)); }
.source .line-close-slash { margin-right: -0.18em; }
/* Half a space before the \`/>\`: a whole one reads as a gap, since the slash leans away from what it
   follows. An id last in the tag keeps its own smaller gap. */
.source .line-close { margin-left: 0.3em; }
.source .id-end + .line-close { margin-left: 0; }
/* The color a string names, as a small square before it, kept with it where a line wraps. */
.source .colored, .source .line-property:has(> .colored) { white-space: nowrap; }
.source .swatch {
  display: inline-block;
  width: 0.75em;
  height: 0.75em;
  margin: 0 0.25em 0 0.1em;
  border-radius: 2px;
  border: 0;
  box-shadow: inset 0 0 0 1px light-dark(rgb(0 0 0 / 0.2), rgb(255 255 255 / 0.25));
  vertical-align: -0.05em;
}
.source .line-equals { color: var(--nx-viewer-equals-color, light-dark(#6e6e6e, #9a9a9a)); }
/* A boolean reads as a mark, a check for true and a cross for false, never as a box to click. */
.source .sheet-boolean { color: var(--nx-viewer-boolean-color, light-dark(#0451a5, #569cd6)); font-weight: 600; }
.source .sheet-boolean.false { color: var(--nx-viewer-boolean-false-color, light-dark(#8e8e8e, #808080)); }
/* A list or a nested sheet sits under its name, a step in, across both columns. */
.source .sheet-below { grid-column: 1 / -1; display: block; padding-left: 2ch; }
/* The icon an element on lines of its own starts with, as a design tool's layers list marks each
   layer: an outline the size of a letter, as quiet as the brackets, before the element's name. */
.source .element-icon {
  display: inline-block;
  width: 1em;
  height: 1em;
  margin-right: 0.6ch;
  vertical-align: -0.15em;
  color: var(--nx-viewer-icon-color, light-dark(#6e7781, #8b949e));
  user-select: none;
}
.source .element-icon > svg { display: block; width: 100%; height: 100%; stroke-width: 1.75; }
/* A group's header bolds its name only, not the attributes written after it. */
.source .sheet-head .line-property { font-weight: 400; }
/* An item written as it is over several lines starts where its group puts it, and its later lines
   keep their indentation from the source, measured from the item's own start. */
.source .sheet-item.as-written *, .source .line-element.alone * { text-indent: 0; }
/* After the reset above, so an item written as it is inside another keeps its own indents. A line
   too long for the pane wraps a little in from where the item starts, not at the far left. */
.source .sheet-item.as-written {
  margin-left: calc(var(--column, 0) * -1ch);
  text-indent: calc((var(--column, 0) + 4) * 1ch) hanging each-line;
}
.source .sheet-item.as-written > .as-written-lead { display: inline-block; width: calc(var(--column, 0) * 1ch); }
/* An element on a line of its own wraps within itself, its later lines a step in from where it starts,
   wherever it sits; and so does one written as its tags on one line inside such an item, as the item's
   other lines wrap from the item's start. One written as its tags over several lines stays inline, its
   lines the item's own: a box of its own would set them in twice, by the box and by their indentation. */
.source .line-element.alone,
.source .sheet-item.as-written :is(.line-element, .tag-line:not(.over-lines)) {
  display: inline-block;
  vertical-align: top;
  /* The reading's width less the element's column: a percentage here would be of a box that is
     itself sized by its content. */
  max-width: calc(100cqw - (var(--line-column, 0) + 1) * 1ch);
  padding-left: 4ch;
  text-indent: -4ch;
  /* At its own spaces, so a wrapped line starts with a word, not inside a string. */
  white-space: pre-wrap;
  overflow-wrap: break-word;
}
/* A sheet's header wraps the same way, its later lines a step in from its icon. */
.source .sheet > .sheet-head { padding-left: 4ch; text-indent: -4ch; white-space: pre-wrap; overflow-wrap: break-word; }
.source .sheet > .sheet-head * { text-indent: 0; }
/* A little room between siblings where either is a sheet, so each group stands apart from the next:
   between the elements of a list or of what an element holds, between sheets that follow one
   another, and around a row whose value is a sheet under its name. Not between one-line elements,
   which read as one list, nor between a sheet's own rows. */
.source .sheet-item:has(> .sheet) + .sheet-item,
.source .sheet-item + .sheet-item:has(> .sheet),
.source .sheet ~ .sheet,
.source .sheet-row:has(> .sheet-below > .sheet) + :is(.sheet-row, .sheet-held),
.source .sheet-row + .sheet-row:has(> .sheet-below > .sheet),
.source .sheet-item.after-blank {
  margin-top: var(--nx-viewer-group-gap, 0.25lh);
}
/* In a narrow pane every value moves under its name, so text never runs down a thin column. */
.source { container-type: inline-size; }
@container (width < 64ch) {
  .source .sheet-row > .sheet-value { grid-column: 1 / -1; padding-left: 2ch; }
  /* An unwritten property with nothing to show takes no line of its own; a click on its name edits it. */
  .source .dimmed-row > .sheet-value:has(> .more-value:empty:only-child) { display: none; }
}

/* A group's name is a step bolder than its rows, and a bracket runs down the group from under its
   name, turning right at its end, as a tree's connectors and an editor's bracket guides do, so each
   group visibly closes. */
.source .sheet-head { font-weight: 600; }
.source .sheet-head .id-value { font-weight: 400; }
.source .sheet { position: relative; }
.source .sheet::after {
  content: "";
  position: absolute;
  left: 0.5ch;
  top: var(--nx-viewer-source-line-height, 18px);
  bottom: 2px;
  width: 7px;
  border-left: 1px solid var(--nx-viewer-rule, light-dark(#0000002e, #ffffff2e));
  border-bottom: 1px solid var(--nx-viewer-rule, light-dark(#0000002e, #ffffff2e));
  border-bottom-left-radius: 3px;
  pointer-events: none;
}
/* A group that starts with an icon drops its bracket from under the icon's middle. */
.source .sheet:has(> .sheet-head > .element-icon)::after { left: calc(0.5em - 0.5px); }
/* A span in another font takes no line height of its own, so every line keeps the height the
   text's own font gives it, whichever fonts meet on it. */
.source .prose, .source .indent { line-height: 0; }
.source .prose {
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, inherit);
}
.source .indent {
  font-family: var(--nx-viewer-indent-font-family, var(--mono));
  font-variation-settings: var(--nx-viewer-indent-font-variation, normal);
}
.source .n.selected { outline: none; background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
/* A group's or a declaration's fold chevron hangs left of where it starts, so folding moves nothing.
   It shows while the pointer is over the reading, and always on what is folded, as an editor's
   folding controls do. */
.source .fold {
  display: inline-block;
  width: 1.2em;
  margin-left: -1.2em;
  text-align: center;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.1s;
  /* Quieter than the icons: a small, thin chevron in a dim gray, as a design tool's layers list
     draws it, that brightens under the pointer. */
  color: var(--nx-viewer-fold-color, light-dark(#8c959f, #7a7a7a));
  user-select: none;
}
/* The chevron is centered on the text's x-height, open or folded. */
.source .fold::before {
  content: "";
  display: inline-block;
  width: 0.28em;
  height: 0.28em;
  border-right: 1px solid currentColor;
  border-bottom: 1px solid currentColor;
  transform: translateY(-0.31em) rotate(45deg);
  transition: transform 0.15s ease;
}
.source .fold.folded::before { transform: translateY(-0.13em) rotate(-45deg); }
/* The chevron of what was just folded or opened starts where it was, and turns to where it is. */
.source .fold.was-folded::before, .source .fold.folded.was-folded::before { transform: translateY(-0.13em) rotate(-45deg); }
.source .fold.was-open::before, .source .fold.folded.was-open::before { transform: translateY(-0.31em) rotate(45deg); }
@media (prefers-reduced-motion: reduce) { .source .fold::before { transition: none; } }
.body:hover .source .fold, .source .fold.folded, .source .fold:focus-visible { opacity: 1; }
.source .fold:hover { color: var(--nx-viewer-foreground, CanvasText); }
/* What a folded unit hides, said quietly after it. */
.source .fold-summary { color: var(--nx-viewer-fold-summary-color, light-dark(#8c959f, #858585)); font-weight: 400; }
/* A folded group is its header on one line: what does not fit of its title is cut, not wrapped. */
.source .sheet.folded > .sheet-head { display: flex; align-items: baseline; white-space: pre; padding-left: 0; text-indent: 0; }
.source .sheet.folded > .sheet-head > * { flex: none; }
.source .sheet.folded > .sheet-head { column-gap: 0; }
.source .sheet.folded > .sheet-head > .element-icon { margin-right: 0.6ch; }
.source .sheet.folded > .sheet-head > .id-value { margin-left: 1ch; }
/* What a folded group hides stands a little apart from its header, as a property does on a line. */
.source .sheet.folded > .sheet-head > .fold-summary { margin-left: 2ch; }
.source .sheet.folded > .sheet-head > .fold-title,
.source .fold-title {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: bottom;
  text-indent: 0;
  font-family: var(--nx-viewer-prose-font-family, inherit);
}
.source .sheet.folded::after { display: none; }
/* A run of supporting declarations: a quiet line that says what it holds, which opens it. */
.source .fold-run-head { cursor: pointer; }
.source .fold-run-head > .fold-summary { font-family: var(--nx-viewer-prose-font-family, inherit); }
.source .fold-run-head:hover > .fold-summary { color: var(--nx-viewer-foreground, CanvasText); }
/* A folded declaration is quieter than what is open, so the eye lands on the document's content. */
.source .fold-declaration.folded { opacity: 0.75; }
/* A unit of the reading (a group, a group's row, an element on one line, a block written as it is) is
   drawn selected and hovered by the highlights behind it, as a list's rows are, not by its own box. */
.source :is(.sheet, .sheet-row, .line-element, .tag-line, .block-line, .sheet-held, .line-property).selected { background: none; }
.source :is(.sheet, .block-line, .line-element, .tag-line) { cursor: default; }
.body:has(> .highlights) { position: relative; isolation: isolate; }
.highlights { position: absolute; inset: 0; pointer-events: none; z-index: -1; }
.highlight { position: absolute; border-radius: 4px; }
.highlight.selected { background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
.highlight.selected-body {
  background: color-mix(in srgb, var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)) 45%, transparent);
}
/* Out of focus, a selection stays, quieter, as a list's does. */
:host(:not(:focus-within)) .highlight.selected { background: var(--nx-viewer-selection-inactive, light-dark(#0000000f, #ffffff17)); }
:host(:not(:focus-within)) .highlight.selected-body {
  background: color-mix(in srgb, var(--nx-viewer-selection-inactive, light-dark(#0000000f, #ffffff17)) 45%, transparent);
}
.highlight.hovered { background: var(--nx-viewer-hover, light-dark(#0000000a, #ffffff0d)); }
/* Over a selection, the row under the pointer lightens it a little. */
.highlight.hovered.in-selection { background: var(--nx-viewer-hover-selected, light-dark(#0000000a, #ffffff12)); }
/* A selected sheet's bracket brightens, as an editor's guide for the block it is in does. */
.source .sheet.selected::after {
  border-color: var(--nx-viewer-selection-guide, light-dark(#0969da99, #4daafcb3));
}

/* The hover card: what a name or a value is, as code in the reading's code face, then its doc in
   its prose face, as an editor's hover shows them. It is placed against the frame, under or over
   what it describes. */
.card {
  position: absolute;
  z-index: 4;
  max-width: min(36rem, 100%);
  max-height: 20em;
  overflow: auto;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  background: var(--nx-viewer-card-background, light-dark(#ffffff, #202020));
  color: var(--nx-viewer-foreground, CanvasText);
  box-shadow: 0 4px 14px light-dark(rgb(0 0 0 / 0.12), rgb(0 0 0 / 0.5));
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, normal);
  font-size: 13px;
  line-height: 1.45;
  white-space: normal;
  overflow-wrap: break-word;
}
.card[hidden] { display: none; }
.card-signature {
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-variation-settings: var(--nx-viewer-source-font-variation, normal);
  font-feature-settings: var(--nx-viewer-source-font-features, normal);
  font-size: var(--nx-viewer-source-font-size, 13px);
  white-space: pre-wrap;
}
.card-doc {
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--nx-viewer-rule, light-dark(#0000001f, #ffffff1f));
}
.card-doc > :first-child { margin-top: 0; }
.card-doc > :last-child { margin-bottom: 0; }
/* What a handle does, after what the item is, quieter. */
.card-hint { margin-top: 4px; color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585)); font-family: var(--nx-viewer-prose-font-family, inherit); }
.card-doc :is(p, ul, ol, pre) { margin: 0 0 0.5em; }
.card-doc :is(ul, ol) { padding-left: 1.4em; }
.card-doc code {
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-size: 0.92em;
  padding: 0 3px;
  border-radius: 3px;
  background: light-dark(#0000000d, #ffffff14);
}
.card-doc pre { padding: 4px 6px; border-radius: 4px; background: light-dark(#0000000a, #ffffff0f); overflow-x: auto; }
.card-doc pre code { padding: 0; background: none; white-space: pre; }
.card-doc a { color: var(--nx-viewer-link-color, light-dark(#0969da, #4daafc)); }

/* Editing, when the host allows it. A value that a click edits shows a faint mark and a text cursor
   under the pointer, or a hand for one that toggles or opens a popover, and a case a chevron, as a
   dropdown does; nothing else changes at rest. */
.body.editable .editable-value { cursor: text; }
.body.editable .editable-value:is(.edit-boolean, .edit-case, .edit-color) { cursor: pointer; }
.body.editable .editable-value:hover:not(.editing) {
  background: var(--nx-viewer-edit-hover, light-dark(#0000000f, #ffffff17));
  border-radius: 3px;
}
/* A boolean toggles on a second click, which would otherwise select the mark as a word. */
.body.editable .edit-boolean { user-select: none; }
/* An edit made and not yet in the host's tree reads muted, as a stale reading does. */
.source .pending { opacity: 0.55; }
/* A field takes its value's own place in the reading, in its font, and grows and wraps with its
   text. It reads as an input: an editor's background inside a focus ring drawn just outside the
   text, so nothing moves, and its own selection color, apart from the selected row's. */
.source .field {
  outline: none;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  border-radius: 2px;
  background: var(--nx-viewer-field-background, light-dark(#ffffff, #1e1e1e));
  box-shadow: 0 0 0 2px var(--nx-viewer-field-background, light-dark(#ffffff, #1e1e1e)),
    0 0 0 3px var(--nx-viewer-field-border, light-dark(#0090f1, #007fd4));
  caret-color: var(--nx-viewer-foreground, CanvasText);
  font-weight: 400;
}
/* A number's field never breaks inside the number. */
.source .field.field-number { white-space: nowrap; overflow-wrap: normal; }
/* Over several lines, a field is one box, as wide as its longest line. */
.source .field.field-lines { display: inline-block; vertical-align: top; }
.source .field::selection {
  background: var(--nx-viewer-field-selection, light-dark(#add6ff, #264f78));
}
.source .field.invalid {
  box-shadow: 0 0 0 2px var(--nx-viewer-field-background, light-dark(#ffffff, #1e1e1e)),
    0 0 0 3px var(--nx-viewer-invalid, light-dark(#cf222e, #f85149));
}
/* A popover opens over the reading, under its value, as the hover card does. */
.popover {
  position: absolute;
  z-index: 5;
  min-width: 12rem;
  max-width: min(28rem, 100%);
  padding: 4px;
  border-radius: 6px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  background: var(--nx-viewer-card-background, light-dark(#ffffff, #202020));
  color: var(--nx-viewer-foreground, CanvasText);
  box-shadow: 0 4px 14px light-dark(rgb(0 0 0 / 0.12), rgb(0 0 0 / 0.5));
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, normal);
  font-size: 13px;
  line-height: 1.45;
}
.popover input:not([type="color"]) {
  width: 100%;
  margin: 0;
  padding: 2px 6px;
  border: 0;
  border-radius: 4px;
  outline: none;
  font: inherit;
  color: inherit;
  background: light-dark(#0000000a, #ffffff0f);
}
.popover.colors { display: flex; gap: 6px; align-items: center; min-width: 0; }
/* After every rule that sets a popover's display, so a closed one never shows. */
.popover[hidden] { display: none; }
/* Why an editor cannot take its entry, under it, in the invalid ring's color. It only says; the
   pointer passes through it to what it covers. */
.edit-problem {
  position: absolute;
  z-index: 6;
  max-width: min(24rem, 100%);
  padding: 2px 6px;
  border-radius: 4px;
  border: 1px solid var(--nx-viewer-invalid, light-dark(#cf222e, #f85149));
  background: var(--nx-viewer-card-background, light-dark(#ffffff, #202020));
  color: var(--nx-viewer-foreground, CanvasText);
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, normal);
  font-size: 12px;
  line-height: 1.4;
  pointer-events: none;
}
.edit-problem[hidden] { display: none; }
.popover .popover-hex { font-family: var(--nx-viewer-source-font-family, var(--mono)); width: 10ch; }
.popover .popover-hex.invalid { box-shadow: inset 0 -1px 0 var(--nx-viewer-invalid, light-dark(#cf222e, #f85149)); }
.popover .popover-picker { width: 28px; height: 22px; padding: 0; border: 0; background: none; cursor: pointer; }
.popover .options { max-height: 16em; overflow: auto; margin-top: 4px; }
.popover .option {
  display: flex;
  flex-direction: column;
  padding: 2px 8px 2px 22px;
  border-radius: 4px;
  cursor: pointer;
  position: relative;
}
.popover .option[hidden] { display: none; }
.popover .option.active { background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
.popover .option.current::before { content: "\\2713"; position: absolute; left: 7px; }
.popover .option-name {
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-size: var(--nx-viewer-source-font-size, 13px);
}
.popover .option-doc {
  color: var(--nx-viewer-fold-summary-color, light-dark(#6e7781, #9a9a9a));
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* A placeholder reads, quieter, in an empty field, and is never written. */
.source .field:empty::before {
  content: attr(data-placeholder);
  color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585));
  pointer-events: none;
}
/* What a selected element does not write: a quiet row that opens a dimmed row to each property. */
.source .more { font-weight: 400; }
.source .more-toggle { cursor: pointer; user-select: none; outline: none; }
.source .more-toggle > .more-label {
  grid-column: 1 / -1;
  color: var(--nx-viewer-more-color, light-dark(#6e7781, #8b949e));
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-size: 0.92em;
}
.source .more-toggle:hover > .more-label, .source .more-toggle:focus-visible > .more-label { color: var(--nx-viewer-foreground, CanvasText); }
.source .more-toggle:focus-visible > .more-label { text-decoration: underline; }
.source .dimmed-row { opacity: var(--nx-viewer-dimmed-opacity, 0.55); outline: none; }
/* A row being set reads at full strength, its field as any other. */
.source .dimmed-row:has(.field) { opacity: 1; }
.source .dimmed-row:focus-visible, .source .added-row:focus-visible { opacity: 1; background: var(--nx-viewer-hover, light-dark(#0000000a, #ffffff0d)); border-radius: 3px; outline: none; }
.source .dimmed-row[hidden], .source .added-row[hidden] { display: none; }
.source .more-default {
  color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585));
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-size: 0.85em;
}
/* An optional property with no default keeps a place for its value, as wide as a short entry, so the
   mark under the pointer and the field it opens are a size to click and type in; it reads the type
   the property takes, quieter. */
.source .more-value:empty, .source .more-value > .field:empty {
  display: inline-block;
  min-width: min(24ch, 100%);
  white-space: nowrap;
  min-height: 1lh;
  vertical-align: top;
}
/* A field opened in an unset property's place keeps the place's width as it fills. */
.source .more-value > .field { display: inline-block; min-width: min(24ch, 100%); vertical-align: top; }
/* An unset uitext property's field types in the prose font its value reads in once written. */
.source .more-value.more-prose > .field:not(:empty) {
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, inherit);
}
.source .more-value:empty::before {
  content: attr(data-placeholder);
  color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585));
}
/* An empty field still reads as one, with room for the caret. */
.source .field:empty { display: inline-block; min-width: 1ch; min-height: 1lh; vertical-align: top; }
.source .more-filter-row > .more-filter {
  grid-column: 1 / -1;
  width: min(24ch, 100%);
  margin: 1px 0;
  padding: 0 6px;
  border: 0;
  border-radius: 4px;
  outline: none;
  font: inherit;
  color: inherit;
  background: light-dark(#0000000a, #ffffff0f);
}
.source .more-filter-row > .more-filter:focus-visible { box-shadow: 0 0 0 1px var(--nx-viewer-field-border, light-dark(#0090f1, #007fd4)); }
/* Under a line, the list sets out as a group's rows do, from where the line's indentation ends. */
.source .more-block {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  column-gap: 2ch;
  justify-content: start;
  padding-left: calc(var(--more-indent, 0) * 1ch);
  white-space: pre;
}
/* A written row's menu: a quiet mark just past its value, or past the tools drawn there, under the
   pointer and when selected, from a point that takes no room, so it moves nothing. */
.source .sheet-row[data-key] { position: relative; }
.source .row-menu {
  position: absolute;
  left: 100%;
  top: 0;
  margin-left: 0.5ch;
  padding: 0 0.4ch;
  border-radius: 3px;
  cursor: pointer;
  user-select: none;
  visibility: hidden;
  color: var(--nx-viewer-fold-color, light-dark(#8c959f, #7a7a7a));
  font-weight: 400;
}
.source .row-anchor { position: relative; display: inline-block; width: 0; height: 1lh; vertical-align: top; }
.source .row-anchor > .row-menu { left: 0; }
.source .sheet-row[data-key]:hover > :is(.sheet-value, .sheet-below) > .row-anchor > .row-menu,
.source .sheet-row.selected > :is(.sheet-value, .sheet-below) > .row-anchor > .row-menu,
.source .row-menu.shared.selected { visibility: visible; }
/* While an item that shares the row's line is selected, the row shows the item's mark alone. */
.source .sheet-row:has(> .row-menu.shared.selected) > :is(.sheet-value, .sheet-below) > .row-anchor > .row-menu { visibility: hidden; }
/* An inline property's mark follows it while it is selected, taking room in the line. */
.source .property-menu { display: none; }
.source .property-menu:has(> .row-menu.selected) { display: inline-block; position: relative; width: 2.5ch; height: 1lh; vertical-align: top; }
.source .property-menu > .row-menu { left: 0; margin-left: 0.25ch; }
/* A line's own mark gives way to the one of a property or an item selected on it, at the same place. */
.source :has(> .item-menu > .row-menu.shared.selected) > .item-menu > .row-menu:not(.shared) { visibility: hidden; }
.source .row-menu:hover { color: var(--nx-viewer-foreground, CanvasText); background: var(--nx-viewer-edit-hover, light-dark(#0000000f, #ffffff17)); }
/* An item's menu hangs past the end of its first line from a point that takes no room. */
.source .item-menu { position: relative; display: inline-block; width: 0; height: 1lh; vertical-align: top; }
.source .item-menu > .row-menu { left: 0; }
/* A folded group's header lines its parts up by their text; the mark's anchor has none, so it takes
   the line's top instead of standing on the text and making the line taller. */
.source .sheet.folded > .sheet-head > .item-menu { align-self: flex-start; }
.source .sheet.selected > .sheet-head > .item-menu > .row-menu:not(.shared),
.source .sheet-head:hover > .item-menu > .row-menu:not(.shared),
.source .line-element.selected > .item-menu > .row-menu:not(.shared),
.source .line-element:hover > .item-menu > .row-menu:not(.shared) { visibility: visible; }
/* An item that moves is dragged by its icon and its name: a hand to grab, and no text selection
   or panning to compete with it. */
.body.editable .drag-handle { cursor: grab; touch-action: none; user-select: none; -webkit-user-select: none; }
/* A touch on a handle's icon may be taken as one on the fold chevron beside it, which then must not
   pan either. */
.body.editable .sheet-head:has(> .drag-handle) > .fold { touch-action: none; }
/* A name is one word: as a box of its own it takes a touch-action, which an inline span does not. */
.body.editable :is(.sheet-kind, .line-kind).drag-handle { display: inline-block; text-indent: 0; }
.body.dragging, .body.dragging * { cursor: grabbing !important; }
/* While dragged, the item stays where it is, muted, and a copy of one line follows the pointer. */
.source .drag-source { opacity: 0.4; }
.drag-ghost {
  position: absolute;
  z-index: 6;
  display: flex;
  align-items: center;
  gap: 0.6ch;
  max-width: 24rem;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  background: var(--nx-viewer-card-background, light-dark(#ffffff, #202020));
  color: var(--nx-viewer-foreground, CanvasText);
  box-shadow: 0 4px 14px light-dark(rgb(0 0 0 / 0.12), rgb(0 0 0 / 0.5));
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-size: var(--nx-viewer-source-font-size, 13px);
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
  opacity: 0.92;
}
.drag-ghost > .element-icon {
  display: inline-block;
  flex: none;
  width: 1em;
  height: 1em;
  color: var(--nx-viewer-icon-color, light-dark(#6e7781, #8b949e));
}
.drag-ghost > .element-icon > svg { display: block; width: 100%; height: 100%; stroke-width: 1.75; }
.drag-ghost > span:last-child { overflow: hidden; text-overflow: ellipsis; }
/* Where the dragged item would land: a line in the selection's color between two items. */
.drop-line {
  position: absolute;
  z-index: 2;
  height: 2px;
  border-radius: 1px;
  background: var(--nx-viewer-drop-line, var(--nx-viewer-selection-guide, light-dark(#0969da, #4daafc)));
  pointer-events: none;
}
.drop-line[hidden] { display: none; }
/* A row's menu opens over the reading, as a popover does. */
.menu {
  position: absolute;
  z-index: 5;
  min-width: 10rem;
  padding: 4px;
  border-radius: 6px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  background: var(--nx-viewer-card-background, light-dark(#ffffff, #202020));
  color: var(--nx-viewer-foreground, CanvasText);
  box-shadow: 0 4px 14px light-dark(rgb(0 0 0 / 0.12), rgb(0 0 0 / 0.5));
  font-family: var(--nx-viewer-prose-font-family, inherit);
  font-variation-settings: var(--nx-viewer-prose-font-variation, normal);
  font-size: 13px;
  line-height: 1.45;
}
.menu[hidden] { display: none; }
.menu-item {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 24px;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  outline: none;
}
/* A default the item resets to, quieter, as the reading draws it. */
.menu-value {
  color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585));
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-size: var(--nx-viewer-source-font-size, 13px);
  white-space: nowrap;
}
.menu-value .swatch {
  display: inline-block;
  width: 0.75em;
  height: 0.75em;
  margin-right: 0.4em;
  border-radius: 2px;
  box-shadow: inset 0 0 0 1px light-dark(rgb(0 0 0 / 0.2), rgb(255 255 255 / 0.25));
  vertical-align: -0.05em;
}
.menu-item:hover, .menu-item:focus { background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
/* Segmented, a few cases read as one row of buttons; the filter takes the keys out of sight. */
.popover.segmented { min-width: 0; }
.popover.segmented > .popover-filter { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); padding: 0; }
.popover.segmented .options { display: flex; flex-wrap: nowrap; gap: 2px; margin-top: 0; max-height: none; overflow: visible; }
.popover.segmented .option {
  padding: 2px 10px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  white-space: nowrap;
}
.popover.segmented .option.current { background: var(--nx-viewer-field-selection, light-dark(#add6ff, #264f78)); }
.popover.segmented .option.current::before, .popover.segmented .option-doc { display: none; }
/* A choices list, Paper-style: each choice's label, then its value, quieter, in a second column; one
   column when no choice has a label. The default's row leads, set off by a rule. */
.popover.choices { min-width: 8rem; }
.popover .choices-list { margin-top: 0; display: grid; grid-template-columns: minmax(0, 1fr); }
.popover .choices-list.labeled { grid-template-columns: max-content minmax(0, 1fr); column-gap: 16px; }
.popover .option.choice {
  display: grid;
  grid-column: 1 / -1;
  grid-template-columns: subgrid;
  align-items: baseline;
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-size: var(--nx-viewer-source-font-size, 13px);
}
.popover .option.choice[hidden] { display: none; }
.popover .choice-value { color: var(--nx-viewer-quiet-color, light-dark(#8c959f, #858585)); white-space: nowrap; }
.popover .choice-value.alone { color: inherit; }
/* With one column, the default's row keeps "Default" and the default on one line. */
.popover .choices-list:not(.labeled) .option.choice-default:not([hidden]) {
  display: flex;
  flex-direction: row;
  align-items: baseline;
  gap: 16px;
}
.popover .choice-default .choice-label { font-family: var(--nx-viewer-prose-font-family, inherit); font-size: 13px; }
.popover .choices-rule {
  grid-column: 1 / -1;
  height: 1px;
  margin: 3px 4px;
  background: var(--nx-viewer-rule, light-dark(#0000001f, #ffffff1f));
}
.popover .choices-rule[hidden] { display: none; }
/* A color's choices lead its popover, over its field and picker. */
.popover.colors.with-choices { flex-wrap: wrap; }
.popover.colors.with-choices > .popover-hex { flex: 1 1 auto; width: auto; min-width: 0; }
.popover.colors .choices-list { flex: 1 0 100%; margin: 0 0 4px; }
.popover .choices-list[hidden] { display: none; }
.popover .option .swatch {
  display: inline-block;
  width: 0.75em;
  height: 0.75em;
  margin-right: 0.4em;
  border-radius: 2px;
  box-shadow: inset 0 0 0 1px light-dark(rgb(0 0 0 / 0.2), rgb(255 255 255 / 0.25));
  vertical-align: -0.05em;
}
/* A number's steppers, Framer-style, and a field's choices chevron: drawn where they would stand in
   the line, just past the value, and taken out of it, so nothing moves. */
.source .value-tools {
  position: absolute;
  z-index: 2;
  display: inline-flex;
  align-items: stretch;
  gap: 2px;
  height: 1lh;
  /* Padding, not a margin: the gap is the tools', so a pointer crossing it keeps them. */
  padding-left: 4px;
  user-select: none;
  white-space: nowrap;
}
/* Taking room in the line, past an open field with more of its line after it. */
.source .value-tools.in-line { position: static; vertical-align: top; }
.source .value-tools .steppers, .source .value-tools .choices-toggle { display: none; }
.source .value-tools.with-steppers .steppers {
  display: inline-flex;
  flex-direction: column;
  gap: 1px;
  align-self: center;
  box-sizing: border-box;
  /* A little taller than the line, so each half is a target a pointer can hit; whole pixels, with the
     rule between them as the gap, so the two halves are exactly the same height. */
  width: 18px;
  height: 25px;
  border: 1px solid var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
  border-radius: 4px;
  overflow: hidden;
  background: var(--nx-viewer-card-border, light-dark(#d0d7de, #454545));
}
/* A stepper is a button: the drag it also takes shows its cursor only once the pointer moves. */
.source .value-tools .stepper {
  position: relative;
  flex: 1 1 0;
  cursor: default;
  background: var(--nx-viewer-field-background, light-dark(#ffffff, #1e1e1e));
}
/* The hover and press tints lie over the field's color, so the rule's color never shows through. */
.source .value-tools .stepper:hover {
  background-image: linear-gradient(var(--nx-viewer-edit-hover, light-dark(#0000000f, #ffffff17)) 0 0);
}
.source .value-tools .stepper:active {
  background-image: linear-gradient(var(--nx-viewer-edit-active, light-dark(#0000001f, #ffffff29)) 0 0);
}
.source .value-tools .stepper::after, .source .value-tools .choices-toggle::after {
  content: "";
  position: absolute;
  inset: 0;
  background: var(--nx-viewer-fold-color, light-dark(#8c959f, #7a7a7a));
  mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E") center / 10px no-repeat;
}
.source .value-tools .stepper.up::after { transform: rotate(180deg); }
.source .value-tools .stepper:hover::after, .source .value-tools .choices-toggle:hover::after {
  background: var(--nx-viewer-foreground, CanvasText);
}
.source .value-tools.with-choices .choices-toggle { display: inline-block; position: relative; width: 12px; cursor: pointer; }
.source .value-tools.open .choices-toggle::after { transform: rotate(180deg); }
.body.stepping, .body.stepping * { cursor: ns-resize !important; user-select: none; }
`;
