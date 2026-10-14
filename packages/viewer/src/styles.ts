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
  --ink-2: light-dark(#4a4c57, #b8b9c2);
  --ink-3: light-dark(#7c7e8a, #8c8e99);
  --rule: light-dark(#dedcd3, #33353f);
  --sheet: light-dark(#ffffff, #1e2027);
  --code-bg: light-dark(#f6f6f2, #1a1c22);
  --blue: var(--nx-viewer-accent, light-dark(#1f4fe0, #7d9bff));
  --blue-soft: light-dark(#e5ecff, #1f2a4d);
  --gold: light-dark(#8a5a00, #ffb020);
  --gold-soft: light-dark(#fff1d1, #3a2d12);
  --teal: light-dark(#0b7285, #5fc9d8);
  --green: light-dark(#1d7a4a, #5ccb8f);
  --green-soft: light-dark(#e2f4ea, #173528);
  --red: light-dark(#b42318, #ff8a7a);
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
button {
  font: inherit;
  font-size: 12px;
  color: inherit;
  background: var(--sheet);
  border: 1px solid var(--rule);
  border-radius: 4px;
  padding: 0 6px;
  cursor: pointer;
}
button:hover, button:focus-visible { border-color: var(--blue); color: var(--blue); }
.document { display: flex; flex-direction: column; gap: 12px; padding: 4px 2px 12px; }
:host([stale]) .document { opacity: 0.55; }
.n { border-radius: 4px; }
.announcer { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
.body:focus-visible { outline: 2px solid var(--blue-soft); outline-offset: 4px; border-radius: 4px; }
.body.by-pointer:focus-visible, .body.by-pointer .card:focus-visible { outline: none; }
.n.selected { outline: 2px solid var(--blue); outline-offset: 2px; }
.n.hovered { background: var(--blue-soft); }

/* The source view: the document's text as an editor shows it. A node is an inline span, so marking
   one changes no line's height. */
.source {
  margin: 0;
  padding: 4px 2px 12px;
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
/* Strings, comments and text may be set in another font, and indentation keeps a fixed width. */
/* Off unless the host sets the \`attribute-spacing\` attribute: an attribute's \`=\` then has an
   eighth of a character of room on each side, however the source spaces it. The room is padding, so
   a selection's tint runs through it, and a space beside the \`=\` is kept in the text at no width. */
:host([attribute-spacing]) .source .attribute-equals { padding: 0 var(--nx-viewer-attribute-equals-space, 0.125ch); }
:host([attribute-spacing]) .source .attribute-space { font-size: 0; }
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
.source .n.selected .uitext-block { background: var(--nx-viewer-selection, light-dark(#0969da1f, #388bfd33)); }
/* A string its viewer formats as an identifier: one quiet token, as products that show ids do,
   in muted gray on a faint tint, with no border and no added height. */
.source .id-value, .value.formatted.id {
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
.source .sheet-value { min-width: 0; }
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
/* A sheet that starts with an icon drops its bracket from under the icon's middle. */
:host([grouping="brackets"]) .source .sheet:has(> .sheet-head > .element-icon)::after { left: calc(0.5em - 0.5px); }
:host([grouping="guides"]) .source .sheet:has(> .sheet-head > .element-icon) { background-position: calc(0.5em - 0.5px) var(--nx-viewer-source-line-height, 18px); }
/* A group's header bolds its name only, not the attributes written after it. */
.source .sheet-head .line-property { font-weight: 400; }
/* An item written as it is over several lines starts where its group puts it, and its later lines
   keep their indentation from the source, measured from the item's own start. */
.source .sheet-item.as-written * { text-indent: 0; }
/* After the reset above, so an item written as it is inside another keeps its own first-line indent. */
.source .sheet-item.as-written {
  margin-left: calc(var(--column, 0) * -1ch);
  text-indent: calc(var(--column, 0) * 1ch);
}
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

/* How a sheet shows what belongs to it, set by the host's grouping attribute.
   outline: names a step bolder, a hairline above each sheet that follows another, a tint over the
   sheet under the pointer, and a rule beside a sheet that is a property's value.
   brackets: names a step bolder, a bracket down each sheet that turns right at its end, and a
   tint over the sheet under the pointer.
   guides: a faint line down each sheet from under its name, as an editor's indent guides. */
:host([grouping="outline"]) .source .sheet-head { font-weight: 600; }
:host([grouping="outline"]) .source .sheet-head .id-value { font-weight: 400; }
:host([grouping="outline"]) .source .sheet { width: calc(100% - var(--indent, 0) * 1ch); }
:host([grouping="outline"]) .source .sheet .sheet { width: 100%; }
:host([grouping="outline"]) .source .sheet-item + .sheet-item > .sheet,
:host([grouping="outline"]) .source .sheet ~ .sheet {
  border-top: 1px solid var(--nx-viewer-rule, light-dark(#0000001f, #ffffff1f));
}
:host([grouping="outline"]) .source .sheet:hover:not(:has(.sheet:hover)) {
  background: var(--nx-viewer-hover, light-dark(#0000000a, #ffffff0a));
  border-radius: 4px;
}
:host([grouping="outline"]) .source .sheet-below > .sheet {
  border-left: 2px solid var(--nx-viewer-rule, light-dark(#0000001f, #ffffff1f));
  padding-left: 1ch;
}
:host([grouping="brackets"]) .source .sheet-head { font-weight: 600; }
:host([grouping="brackets"]) .source .sheet-head .id-value { font-weight: 400; }
:host([grouping="brackets"]) .source .sheet { position: relative; }
/* A bracket down the sheet from under its name, turning right at its end, as a tree's connectors
   and an editor's bracket guides do, so each sheet visibly closes. */
:host([grouping="brackets"]) .source .sheet::after {
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
:host([grouping="brackets"]) .source .sheet:hover:not(:has(.sheet:hover)) {
  background: var(--nx-viewer-hover, light-dark(#0000000a, #ffffff0a));
  border-radius: 4px;
}
:host([grouping="guides"]) .source .sheet {
  background-image: linear-gradient(var(--nx-viewer-rule, light-dark(#00000024, #ffffff24)), var(--nx-viewer-rule, light-dark(#00000024, #ffffff24)));
  background-size: 1px calc(100% - var(--nx-viewer-source-line-height, 18px));
  background-position: 0.5ch var(--nx-viewer-source-line-height, 18px);
  background-repeat: no-repeat;
}
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
.source .n.hovered { background: none; }

.details { border: 1px solid var(--rule); border-radius: 6px; padding: 2px 10px; font-size: 13px; color: var(--ink-2); }
.document > .details { background: var(--sheet); border-radius: 8px; padding: 4px 12px; }
.details summary { cursor: pointer; }
.details-body { display: flex; flex-direction: column; gap: 4px; padding: 4px 0 6px; }

.viewer-element { display: flex; flex-direction: column; gap: 2px; }
.viewer-line { display: flex; flex-wrap: wrap; gap: 2px 14px; align-items: baseline; }
.viewer-kind { font-family: var(--mono); font-size: 12.5px; font-weight: 600; color: var(--blue); }
.viewer-attribute > .label { color: var(--ink-3); font-size: 13px; cursor: help; }
.viewer-attribute.subject { min-width: 11ch; }
.viewer-nested, .viewer-held { display: flex; flex-direction: column; gap: 2px; }
.viewer-nested { padding-left: 14px; }
.case-line { display: inline-flex; flex-wrap: wrap; align-items: baseline; }
.declaration-sign { color: var(--ink-3); }
.declaration-docs { display: flex; flex-direction: column; gap: 4px; }
.case-bar { color: var(--ink-3); white-space: pre; }
.value.boolean.written.true, .value.boolean.written.false { font-family: var(--mono); font-size: 12.5px; font-weight: 400; color: inherit; }

.declaration { display: flex; flex-direction: column; gap: 6px; background: var(--sheet); border: 1px solid var(--rule); border-radius: 8px; padding: 10px 12px; }
.declaration-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: baseline; }
.declaration-kind { font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--ink-3); }
.declaration-name { font-weight: 600; font-family: var(--mono); font-size: 13px; }
/* The Source button keeps the header's top right corner, so a long title wraps beside it and the
   hidden button never takes a line of its own. */
.declaration-header, .card-header { position: relative; padding-right: 60px; }
.declaration-header .source-toggle, .card-header .source-toggle { position: absolute; top: 0; right: 0; opacity: 0; transition: opacity 0.1s; }
/* A Source button shows for the innermost card or declaration under the pointer, for the selected
   one, and when it has focus. Where nothing hovers, as on a touch screen, every button shows. */
.declaration:hover:not(:has(.card:hover)) > .declaration-header > .source-toggle,
.card:hover:not(:has(.card:hover)) > .card-header > .source-toggle,
.n.selected > .declaration-header > .source-toggle,
.n.selected > .card-header > .source-toggle,
.source-toggle:focus-visible { opacity: 1; }
@media (hover: none) { .declaration-header .source-toggle, .card-header .source-toggle { opacity: 0.7; } }
.declaration-body { display: grid; grid-template-columns: max-content max-content minmax(0, 1fr); gap: 1px 16px; padding-left: 16px; }
.declaration-body > :not(.member), .nested > :not(.member) { grid-column: 1 / -1; }
.declaration > .doc { color: var(--ink-2); }
.declaration-body > .type { justify-self: start; }
.group-label { font-size: 12px; color: var(--ink-3); margin-top: 2px; }
.member { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; align-items: baseline; line-height: 1.4; }
.member.emit { display: flex; flex-wrap: wrap; gap: 2px 6px; }
.case-line > .member.union-case { display: inline-flex; }
.case-head { white-space: nowrap; }
.case-head > .case-bar { margin-right: 8px; }
.case-name { font-family: var(--mono); font-size: 13px; font-weight: 500; }
.case-fields { grid-column: 2 / -1; display: inline-flex; flex-wrap: wrap; gap: 0 6px; align-items: baseline; }
.union-case > .nested:not(.inline-fields) { grid-column: 1 / -1; }
/* One-line cases sit as close as a record's rows. A case with rows of fields under it stands a
   little apart, so its fields read as its own. */
.member.union-case:has(> .nested:not(.inline-fields)) { margin-block: 3px; }
.brace { color: var(--ink-3); }
.nested.inline-fields { flex-basis: auto; padding-left: 0; display: inline-flex; flex-wrap: wrap; gap: 0 16px; }
.inline-fields > .member { display: inline-flex; gap: 8px; }
.inline-fields > .member > :empty { display: none; }
.union-case > .nested:not(.inline-fields) { padding-left: 30px; }
.member-name { font-family: var(--mono); font-size: 13px; font-weight: 500; }
.member-default-label { font-size: 12px; color: var(--ink-3); }
.member-qualifier { font-size: 12px; color: var(--ink-3); }
.nested { flex-basis: 100%; padding-left: 14px; display: grid; grid-template-columns: max-content max-content minmax(0, 1fr); gap: 1px 16px; }
code, .type { font-family: var(--mono); font-size: 12.5px; }
.type { color: var(--teal); }
.tag { font-size: 11px; color: var(--ink-3); border: 1px solid var(--rule); border-radius: 4px; padding: 0 5px; }

.card { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
/* A declaration is the card. An element inside one is a block: a rule over it when it follows
   another, and a rule beside it when another element holds it. */
.document > .card { background: var(--sheet); border: 1px solid var(--rule); border-radius: 8px; padding: 10px 12px; }
.card + .card { border-top: 1px solid var(--rule); border-radius: 0; padding-top: 8px; }
.card .card { border-top: none; border-left: 2px solid var(--rule); border-radius: 0; padding: 0 0 0 12px; }
.card:focus-visible { outline: 2px solid var(--blue); }
.card-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.kind { font-size: 12px; font-weight: 600; color: var(--blue); background: var(--blue-soft); border-radius: 999px; padding: 1px 10px; }
.kind.unresolved { color: var(--red); background: transparent; border: 1px dashed var(--red); font-family: var(--mono); }
.unresolved-marker { font-size: 11px; color: var(--red); }
.card-name { font-family: var(--mono); font-size: 12px; color: var(--ink-3); }
.rows { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 1px 14px; align-items: baseline; line-height: 1.4; }
.rows > :not(.row) { grid-column: 1 / -1; }
.rows.inline-rows { display: inline-flex; flex-wrap: wrap; gap: 0 16px; min-width: 0; }
.inline-rows > .row { display: inline-flex; align-items: baseline; }
.equals { color: var(--ink-3); }
.row { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; align-items: baseline; }
.row > .label, .row > .when { color: var(--ink-3); font-size: 13px; }
.row > .label, .row > .when { cursor: help; }
.row.content-row > .label, .row.handler > .when { align-self: start; }
.row-value { min-width: 0; overflow-wrap: anywhere; }
.row.ghost > .label, .ghost-value { color: var(--ink-3); font-style: italic; }
.ghost-value { font-family: var(--mono); font-size: 12.5px; margin-right: 6px; }
/* Elements that follow one another share one column for their rows' names, so their values line up. */
.item-list, .content, .sequence.blocks { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 6px 14px; }
.item-list > *, .content > *, .sequence.blocks > * { grid-column: 1 / -1; }
.item-list > .card, .content > .card, .sequence.blocks > .card { display: grid; grid-template-columns: subgrid; row-gap: 4px; }
.item-list > .card > *, .content > .card > *, .sequence.blocks > .card > * { grid-column: 1 / -1; }
.item-list > .card > .rows, .content > .card > .rows, .sequence.blocks > .card > .rows { grid-template-columns: subgrid; }
.card.inline-layout { display: inline-flex; vertical-align: baseline; padding: 1px 8px; gap: 4px; border: 1px solid var(--rule); border-radius: 6px; }
.part.title { font-weight: 600; }
.part.subtitle { color: var(--ink-2); font-size: 13px; }
.card:not(.row-layout):not(.inline-layout) > .card-header > .part.subtitle { order: 1; flex-basis: 100%; }
.part.badge { font-size: 11px; color: var(--ink-2); border: 1px solid var(--rule); border-radius: 999px; padding: 0 8px; }
.part.image .picture-url { display: none; }
.part, .phrase { cursor: help; }
.row.phrase-row > .row-value { grid-column: 1 / -1; }
.value.phrase { color: var(--ink-2); font-weight: 500; }
.ghost-value.phrase { font-family: inherit; font-size: inherit; }
.picture { max-width: 96px; max-height: 64px; border-radius: 4px; vertical-align: middle; margin-right: 6px; }
.part.image .picture { max-height: 32px; margin-right: 0; }
.picture-url { font-size: 12px; color: var(--ink-3); }
.swatch { display: inline-block; width: 12px; height: 12px; border-radius: 3px; border: 1px solid var(--rule); vertical-align: -1px; margin-right: 6px; }
.value.formatted a { color: var(--blue); }
.value.formatted.markdown > .text-body { display: inline-flex; }
.pill.case.labelled { font-family: inherit; cursor: help; }

.value.string { color: var(--green); }
.value.number { font-variant-numeric: tabular-nums; font-family: var(--mono); font-size: 12.5px; }
.value.boolean { font-weight: 700; }
.value.boolean.true { color: var(--green); }
.value.boolean.false { color: var(--red); }
.value.empty { color: var(--ink-3); font-style: italic; }
.pill { display: inline-block; font-family: var(--mono); font-size: 12px; border: 1px solid var(--rule); border-radius: 999px; padding: 0 8px; }
.pill.case { color: var(--blue); border-color: var(--blue); }
.pill.current { color: var(--blue); border-color: var(--blue); background: var(--blue-soft); font-weight: 600; }
.qualifier { color: var(--ink-3); }
.sequence { display: inline; }
.sequence > .reference { display: inline-block; vertical-align: top; }
.sequence.blocks { align-items: stretch; }

.reference .ref { font-family: var(--mono); font-size: 12.5px; color: var(--blue); border-bottom: 1px dashed var(--blue); cursor: pointer; }
.expand { margin-left: 4px; padding: 0 4px; font-size: 11px; line-height: 1.3; }
.inherited-toggle { padding: 0 4px; font-size: 11px; line-height: 1.3; }
/* What a declaration inherits, folded until its toggle is pressed: the same columns as its own
   members, a little quieter, since none of it is written here. */
.inherited { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; row-gap: 1px; padding-bottom: 4px; }
.inherited[hidden] { display: none; }
.inherited > :not(.member) { grid-column: 1 / -1; }
.inherited-member > .member-name { font-weight: 400; color: var(--ink-2); }
.inherited-default { font-family: var(--mono); font-size: 12.5px; }
.expansion { display: block; margin: 6px 0 2px; padding-left: 10px; border-left: 2px solid var(--blue-soft); }
.member-access, .call, .binding { font-family: var(--mono); font-size: 12.5px; }
.member-access .member-name { font-weight: 400; }
.dot { color: var(--ink-3); }
.op { color: var(--ink-2); }
.slot { color: var(--gold); background: var(--gold-soft); border-radius: 4px; padding: 0 4px; font-family: var(--mono); font-size: 12.5px; }
.sentence .words { color: inherit; }

.condition, .match, .loop { display: flex; flex-direction: column; gap: 4px; }
.when-label { color: var(--gold); font-size: 13px; font-weight: 500; }
.when-label .verb, .otherwise, .verb { color: var(--gold); }
.otherwise { font-size: 13px; font-weight: 500; }
.branch { display: flex; flex-direction: column; gap: 6px; padding-left: 12px; border-left: 2px solid var(--gold-soft); }
.arm { display: grid; grid-template-columns: minmax(0, max-content) minmax(0, 1fr); gap: 4px 12px; align-items: baseline; padding: 2px 0; border-top: 1px solid var(--rule); }
.arm-when { color: var(--ink-2); font-size: 13px; }
.update { display: flex; flex-direction: column; gap: 2px; }
.set-line .slot-name { font-family: var(--mono); font-size: 12.5px; color: var(--gold); }
.arrow { color: var(--ink-3); }

.comment { color: var(--ink-3); font-size: 13px; font-style: italic; white-space: pre-wrap; }
.notes { display: flex; flex-direction: column; }
.doc { color: var(--ink-2); font-size: 13.5px; }
.doc p, .text-body p { margin: 0; }
.text-body { display: flex; flex-direction: column; gap: 6px; }
.text-body h3, .text-body h4, .text-body h5, .text-body h6 { margin: 0; font-size: 15px; }
.text-body ul, .text-body ol { margin: 0; padding-left: 1.3em; }
.text-body pre.raw { margin: 0; font-family: var(--mono); font-size: 12.5px; white-space: pre-wrap; }
.card > .content > .text-body { border-left: 3px solid var(--gold); padding-left: 10px; }
.unparsed { border: 1px dashed var(--red); border-radius: 6px; padding: 4px 8px; }
.unparsed-marker { font-size: 11px; color: var(--red); }
.unparsed pre { margin: 0; font-family: var(--mono); font-size: 12.5px; white-space: pre-wrap; }
.import { font-size: 13px; }

.hover, .source-panel {
  position: absolute;
  z-index: 3;
  max-width: min(420px, 100%);
  background: var(--sheet);
  border: 1px solid var(--rule);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(22, 23, 28, 0.14);
  padding: 8px 10px;
  font-size: 13px;
}
.hover > div + div { margin-top: 6px; }
.hover-cases { display: flex; flex-wrap: wrap; gap: 4px; }
.hover-note { color: var(--ink-3); font-size: 12px; }
.source-panel { max-width: 100%; }
.source-bar { display: flex; justify-content: space-between; gap: 8px; align-items: center; font-size: 12px; color: var(--ink-3); margin-bottom: 4px; }
.source-panel pre { margin: 0; font-family: var(--mono); font-size: 12.5px; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 50vh; overflow: auto; }
`;
