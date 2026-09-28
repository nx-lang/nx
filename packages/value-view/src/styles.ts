/**
 * The element's stylesheet. Colors come from the page through custom properties, and token colors
 * follow the page's `color-scheme` through `light-dark()`.
 */
export const styles = `
:host {
  display: block;
  position: relative;
  font-family: var(--nx-value-font-family, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
  font-size: var(--nx-value-font-size, 13px);
  line-height: var(--nx-value-line-height, 1.5);
  color: var(--nx-value-foreground, CanvasText);
  background: var(--nx-value-background, transparent);
}
:host([hidden]) { display: none; }
.frame { position: relative; }
.toolbar {
  position: absolute;
  top: 4px;
  right: 4px;
  display: flex;
  gap: 6px;
  align-items: center;
  z-index: 1;
}
.badge {
  font: 11px/1.6 system-ui, sans-serif;
  padding: 0 6px;
  border-radius: 4px;
  background: var(--nx-value-badge-background, light-dark(#fff4d6, #3d3212));
  color: var(--nx-value-badge-foreground, light-dark(#6b4e00, #f2d27a));
}
.copy {
  font: 12px/1.6 system-ui, sans-serif;
  padding: 1px 8px;
  border: 1px solid var(--nx-value-border, light-dark(#d0d7de, #3d444d));
  border-radius: 4px;
  background: var(--nx-value-button-background, light-dark(#f6f8fa, #21262d));
  color: inherit;
  cursor: pointer;
  opacity: 0.85;
}
.copy:hover, .copy:focus-visible { opacity: 1; }
/* Copy shows while the value is pointed at or focused, so it never sits over the first line. */
@media (hover: hover) {
  .copy { visibility: hidden; }
  :host(:hover) .copy, .frame:focus-within .copy { visibility: visible; }
}
pre {
  margin: 0;
  padding: var(--nx-value-padding, 8px 12px 8px 24px);
  overflow: auto;
  white-space: pre;
  font: inherit;
  outline: none;
}
pre:focus-visible { box-shadow: inset 0 0 0 2px var(--nx-value-focus, Highlight); }
code { font: inherit; }
:host([stale]) code { opacity: 0.55; }
.t { color: light-dark(var(--l, inherit), var(--d, inherit)); }
.i { font-style: italic; }
.b { font-weight: bold; }
.u { text-decoration: underline; }
.node { position: relative; border-radius: 2px; }
.node.hovered, .node:focus { background: var(--nx-value-hover-background, light-dark(#0969da1a, #388bfd26)); outline: none; }
.node.declared.hovered { cursor: pointer; }
.fold {
  position: absolute;
  left: -1.3em;
  top: 0;
  width: 1.2em;
  height: 1.5em;
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  opacity: 0.45;
  cursor: pointer;
  font: inherit;
  line-height: inherit;
}
.fold::before { content: "\\25BE"; }
.fold[aria-expanded="false"]::before { content: "\\25B8"; }
.fold:hover { opacity: 1; }
.ellipsis {
  padding: 0 4px;
  border: 1px solid var(--nx-value-border, light-dark(#d0d7de, #3d444d));
  border-radius: 3px;
  background: var(--nx-value-button-background, light-dark(#f6f8fa, #21262d));
  color: inherit;
  font: inherit;
  line-height: 1.1;
  cursor: pointer;
}
.notice {
  margin: 0;
  padding: 4px 12px 8px 24px;
  font: 12px/1.5 system-ui, sans-serif;
  opacity: 0.8;
}
.hover {
  position: absolute;
  z-index: 2;
  max-width: min(36em, 90%);
  padding: 4px 8px;
  border: 1px solid var(--nx-value-border, light-dark(#d0d7de, #3d444d));
  border-radius: 6px;
  background: var(--nx-value-hover-popup-background, light-dark(#ffffff, #161b22));
  color: var(--nx-value-foreground, CanvasText);
  box-shadow: 0 4px 12px light-dark(#1f232826, #01040966);
  font: 12px/1.5 system-ui, sans-serif;
  pointer-events: none;
}
.hover p { margin: 4px 0; }
.hover code { font-family: var(--nx-value-font-family, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace); }
.hover .hover-code { margin: 2px 0; padding: 0; white-space: pre-wrap; font-size: 12px; }
[hidden] { display: none !important; }
`;
