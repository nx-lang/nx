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
.toolbar {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  align-items: center;
  padding: 4px 0;
  min-height: 0;
}
.toolbar:empty { display: none; }
.badge {
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
  padding: 4px 12px 12px 2px;
  font-family: var(--nx-viewer-source-font-family, var(--mono));
  font-variation-settings: var(--nx-viewer-source-font-variation, normal);
  font-feature-settings: var(--nx-viewer-source-font-features, normal);
  font-size: var(--nx-viewer-source-font-size, 13px);
  line-height: var(--nx-viewer-source-line-height, 18px);
  white-space: pre;
  tab-size: 2;
}
:host([stale]) .source { opacity: 0.55; }
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
/* A string its viewer formats as an identifier: one quiet token, as products that show ids do,
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
/* The color a string names, as a small square before it. */
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
   wherever it sits; and so does one written as its tags inside such an item, as the item's other lines
   wrap from the item's start. */
.source .line-element.alone,
.source .sheet-item.as-written :is(.line-element, .tag-line) {
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
/* A unit of the reading (a group, a group's row, an element on one line, a block written as it is) is
   drawn selected and hovered by the highlights behind it, as a list's rows are, not by its own box. */
.source :is(.sheet, .sheet-row, .line-element, .tag-line, .block-line, .sheet-held).selected { background: none; }
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
`;
