/**
 * A DOM for the tests: jsdom, installed as the globals the element module reads when it loads.
 * Import this before the element module.
 */
import { JSDOM } from "jsdom";

export const dom = new JSDOM("<!doctype html><html><body></body></html>", {
  pretendToBeVisual: true
});

const window = dom.window;
const globals: Record<string, unknown> = {
  window,
  document: window.document,
  HTMLElement: window.HTMLElement,
  customElements: window.customElements,
  Node: window.Node,
  CustomEvent: window.CustomEvent,
  getComputedStyle: window.getComputedStyle.bind(window)
};
for (const [name, value] of Object.entries(globals)) {
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
}

export const document = window.document;
