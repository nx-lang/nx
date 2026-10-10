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
.n.selected { outline: 2px solid var(--blue); outline-offset: 2px; }
.n.hovered { background: var(--blue-soft); }

.details { border: 1px solid var(--rule); border-radius: 6px; padding: 2px 10px; font-size: 13px; color: var(--ink-2); }
.details summary { cursor: pointer; }
.details-body { display: flex; flex-direction: column; gap: 4px; padding: 4px 0 6px; }

.declaration { display: flex; flex-direction: column; gap: 6px; }
.declaration-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: baseline; }
.declaration-kind { font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--ink-3); }
.declaration-name { font-weight: 600; font-family: var(--mono); font-size: 13px; }
.declaration-header .source-toggle, .card-header .source-toggle { margin-left: auto; opacity: 0.7; }
.declaration-body { display: flex; flex-direction: column; gap: 4px; padding-left: 12px; border-left: 2px solid var(--rule); }
.group-label { font-size: 12px; color: var(--ink-3); margin-top: 2px; }
.member { display: flex; flex-wrap: wrap; gap: 6px; align-items: baseline; }
.member-name { font-family: var(--mono); font-size: 13px; font-weight: 500; }
.member-default-label { font-size: 12px; color: var(--ink-3); }
.nested { flex-basis: 100%; padding-left: 14px; display: flex; flex-direction: column; gap: 2px; }
code, .type { font-family: var(--mono); font-size: 12.5px; }
.type { color: var(--ink-2); background: var(--code-bg); border: 1px solid var(--rule); border-radius: 4px; padding: 0 4px; }
.tag { font-size: 11px; color: var(--ink-3); border: 1px solid var(--rule); border-radius: 4px; padding: 0 5px; }

.card { background: var(--sheet); border: 1px solid var(--rule); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
.card:focus-visible { outline: 2px solid var(--blue); }
.card-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.kind { font-size: 12px; font-weight: 600; color: var(--blue); background: var(--blue-soft); border-radius: 999px; padding: 1px 10px; }
.kind.unresolved { color: var(--red); background: transparent; border: 1px dashed var(--red); font-family: var(--mono); }
.unresolved-marker { font-size: 11px; color: var(--red); }
.card-name { font-family: var(--mono); font-size: 12px; color: var(--ink-3); }
.rows { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 4px 14px; align-items: baseline; }
.rows > :not(.row) { grid-column: 1 / -1; }
.row { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; align-items: baseline; }
.row > .label, .row > .when { color: var(--ink-3); font-size: 13px; }
.row > .label, .row > .when { cursor: help; }
.row.content-row > .label, .row.handler > .when { align-self: start; }
.row-value { min-width: 0; overflow-wrap: anywhere; }
.row.ghost > .label, .ghost-value { color: var(--ink-3); font-style: italic; }
.ghost-value { font-family: var(--mono); font-size: 12.5px; margin-right: 6px; }
.item-list { display: flex; flex-direction: column; gap: 6px; }
.content { display: flex; flex-direction: column; gap: 6px; }

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
.sequence { display: inline-flex; flex-wrap: wrap; gap: 4px 8px; align-items: baseline; }
.sequence.blocks { display: flex; flex-direction: column; align-items: stretch; gap: 6px; }

.reference .ref { font-family: var(--mono); font-size: 12.5px; color: var(--blue); border-bottom: 1px dashed var(--blue); cursor: pointer; }
.expand { margin-left: 4px; padding: 0 4px; font-size: 11px; line-height: 1.3; }
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
