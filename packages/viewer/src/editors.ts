/**
 * The pieces of the reading's editors: where in a value's rendering its text sits, the value shown
 * once an edit is made and before the host answers, a number's steps, and the popovers' content.
 */
import type { NamedValue, SourceCase, SourceEditorHints, SourceValueFormat } from "@nx-lang/language-protocol";

import { isNumericLiteral, type EditTarget, type TypeEditor } from "./edit.js";
import { decodeString } from "./source.js";

/** A boolean's mark, as the reading draws it. */
export const BOOLEAN_MARKS = { true: "✓", false: "✗" } as const;

/**
 * Where a value's text sits in its rendering: the element holding it, the quotes drawn around it
 * there, and what else it holds that stays, as a color's swatch.
 */
export interface ValueHost {
  readonly host: HTMLElement;
  readonly lead: Node | undefined;
  readonly trail: Node | undefined;
  readonly keep: readonly Element[];
}

/**
 * Where the text of the value drawn by `span`, written as `written`, sits: in its words' box for UI
 * text, or in the span itself, with the quotes the reading draws, kept as they are drawn.
 */
export function valueHost(span: HTMLElement, written: string): ValueHost {
  const document = span.ownerDocument;
  const host = span.classList.contains("uitext-body") ? span : (span.querySelector<HTMLElement>(".uitext-body") ?? span);
  const keep = [...host.children].filter((child) => child.classList.contains("swatch"));
  // Which quotes the reading draws around the text is found once, from the rendering as drawn, and
  // kept on it, since a value shown since no longer reads as written.
  let quotes = span.dataset["quotes"];
  if (quotes === undefined) {
    const shown = host.textContent ?? "";
    const quoted = written.length >= 2 && written.startsWith('"') && written.endsWith('"');
    // The reading draws a string's quote and backslash escapes as what they escape, except in a
    // string drawn as code, between its own quotes.
    const read = quoted ? `"${decodeString(written) ?? written.slice(1, -1)}"` : written;
    quotes =
      quoted && shown === written
        ? "both"
        : quoted && (shown === written.slice(1) || shown === read.slice(1))
          ? "trail"
          : "none";
    span.dataset["quotes"] = quotes;
  }
  const both = quotes === "both";
  const trailing = both || quotes === "trail";
  const quote = (node: ChildNode | null): Node =>
    node !== null && node.nodeType === 1 && /quote/.test((node as Element).className)
      ? node
      : document.createTextNode('"');
  return {
    host,
    lead: both ? quote(host.firstChild) : undefined,
    trail: trailing ? quote(host.lastChild) : undefined,
    keep,
  };
}

/**
 * Shows `value` in place of what `span` draws for `target`, muted as waiting for the host: a mark
 * for a boolean, a case's name after its qualifier, and otherwise the text between the quotes the
 * reading draws.
 */
export function showValue(span: HTMLElement, target: EditTarget, written: string, value: string): void {
  span.classList.add("pending");
  if (target.kind === "boolean") {
    const on = value === "true";
    span.textContent = on ? BOOLEAN_MARKS.true : BOOLEAN_MARKS.false;
    span.classList.toggle("true", on);
    span.classList.toggle("false", !on);
    span.title = value;
    span.setAttribute("aria-label", value);
    return;
  }
  if (target.kind === "case") {
    const shown = span.textContent ?? "";
    const dot = shown.lastIndexOf(".");
    span.replaceChildren(`${dot < 0 ? "" : shown.slice(0, dot + 1)}${value}`);
    return;
  }
  const { host, lead, trail, keep } = valueHost(span, written);
  for (const swatch of keep) {
    (swatch as HTMLElement).style.backgroundColor = value.trim();
  }
  // A string drawn as code, between its own quotes, shows its quotes and backslashes escaped, as the
  // reading draws it.
  const shown = lead !== undefined && target.kind === "text" ? value.replace(/[\\"]/g, "\\$&") : value;
  host.replaceChildren(...keep, ...(lead === undefined ? [] : [lead]), shown, ...(trail === undefined ? [] : [trail]));
}

/**
 * `text`, a numeric literal, moved by `steps` of `hints.step`, or of one: an integer stays one, and a
 * real keeps as many decimals as it or the step has; it stays within `hints.min` and `hints.max`,
 * and unless `signed`, it stops at zero. Nothing when `text` is not a decimal number.
 */
export function stepNumber(text: string, steps: number, signed = true, hints?: SourceEditorHints): string | undefined {
  const trimmed = text.trim();
  if (!isNumericLiteral(trimmed) || /[xXeE]/.test(trimmed)) {
    return undefined;
  }
  const step = hints?.step !== undefined && hints.step > 0 ? hints.step : 1;
  const decimals = Math.max(decimalsOf(trimmed), decimalsOf(String(step)));
  let next = Number(trimmed) + steps * step;
  // Where a number cannot be negative, it stops at zero.
  if (!signed) {
    next = Math.max(0, next);
  }
  next = clamp(next, hints);
  return decimals === 0 ? String(Math.round(next)) : next.toFixed(decimals);
}

/** How many decimals a number is written with. */
function decimalsOf(text: string): number {
  return text.includes(".") ? text.length - text.indexOf(".") - 1 : 0;
}

/** `value` kept within the range `hints` gives, if any. */
function clamp(value: number, hints: SourceEditorHints | undefined): number {
  const low = hints?.min ?? -Infinity;
  const high = hints?.max ?? Infinity;
  return low > high ? value : Math.min(high, Math.max(low, value));
}

/**
 * What a number's field holds, as the number to write: trimmed, and without the unit its hints
 * give, which a reader may type after it, as `16px`.
 */
export function numberEntry(text: string, hints: SourceEditorHints | undefined): string {
  const trimmed = text.trim();
  const unit = hints?.unit?.trim();
  return unit !== undefined && unit !== "" && trimmed.endsWith(unit) ? trimmed.slice(0, -unit.length).trim() : trimmed;
}

/** Why the number `entry` is outside the range its hints give, as `At most 64`, or nothing. */
export function rangeProblem(entry: string, hints: SourceEditorHints | undefined): string | undefined {
  const value = Number(entry);
  if (hints === undefined || !Number.isFinite(value)) {
    return undefined;
  }
  const unit = hints.unit === undefined ? "" : ` ${hints.unit}`;
  if (hints.min !== undefined && hints.max !== undefined && hints.min > hints.max) {
    // An empty range bounds nothing.
    return undefined;
  }
  if (hints.min !== undefined && value < hints.min) {
    return `At least ${hints.min}${unit}`;
  }
  if (hints.max !== undefined && value > hints.max) {
    return `At most ${hints.max}${unit}`;
  }
  return undefined;
}

/** Whether a number edits with a slider: its hints ask for one, or give a range and ask for nothing. */
export function isSlider(target: EditTarget): boolean {
  const hints = target.hints;
  if (target.kind !== "number" || hints === undefined) {
    return false;
  }
  if (hints.min !== undefined && hints.max !== undefined && hints.min > hints.max) {
    return false;
  }
  return hints.kind === "slider" || (hints.kind === undefined && hints.min !== undefined && hints.max !== undefined);
}

/**
 * The number a slider at `position` stands for, written as the number it started from is: snapped
 * to the nearest of its hints' stops unless `free`, and to its step.
 */
export function slidValue(position: number, hints: SourceEditorHints, from: string, free: boolean): string {
  let value = position;
  const stops = hints.stops ?? [];
  if (!free && stops.length > 0) {
    value = stops.reduce((best, stop) => (Math.abs(stop - position) < Math.abs(best - position) ? stop : best), stops[0]!);
  } else if (hints.step !== undefined && hints.step > 0) {
    const base = hints.min ?? 0;
    value = base + Math.round((position - base) / hints.step) * hints.step;
  }
  value = clamp(value, hints);
  const decimals = Math.max(decimalsOf(from.trim()), hints.step === undefined ? 0 : decimalsOf(String(hints.step)));
  // Between stops, with no step, a real keeps two decimals at most and an integer stays one.
  if (free && stops.length > 0 && hints.step === undefined && decimals === 0) {
    return String(Math.round(value));
  }
  return decimals === 0 ? String(Math.round(value)) : value.toFixed(decimals);
}

/**
 * The stop a slider's arrow keys go to from `current`: the next one up or down among its hints'
 * stops within `min` to `max`. Nothing past the last stop either way.
 */
export function nextStop(current: number, direction: 1 | -1, hints: SourceEditorHints, min: number, max: number): number | undefined {
  const stops = [...new Set(hints.stops ?? [])].filter((stop) => stop >= min && stop <= max).sort((a, b) => a - b);
  return direction === 1 ? stops.find((stop) => stop > current) : stops.reverse().find((stop) => stop < current);
}

/** A mark under a slider's track: a stop, a named value's number, or the property's default. */
export interface SliderMark {
  /** Where on the track, from 0 at its start to 1 at its end. */
  readonly at: number;
  readonly value: number;
  readonly kind: "stop" | "named" | "default";
  /** The named values the mark stands for, the first of which its label shows. */
  readonly references?: readonly string[];
}

/** A label under a slider's track: a stop's number or a named value's path. */
export interface SliderLabel {
  readonly at: number;
  readonly value: number;
  readonly text: string;
  /** The named value a click on the label chooses. */
  readonly reference?: string;
}

/** About how wide a character of a slider's labels is, and the least room kept between two. */
const LABEL_CHARACTER = 6.6;
const LABEL_GAP = 6;
/** With more stops than this, only the first and last stops are labeled with their numbers. */
const LABELED_STOPS = 7;

/**
 * A slider's marks and labels: a tick at each stop and at each named value whose literal number is
 * in range, a notch at `fallback`, its default, when that is a number in range. Labels give the
 * first stop's number, the named values' paths, the last stop's number, then the other stops' numbers
 * when there are few, leaving out any that would overlap one already placed on a track `width` pixels
 * wide.
 */
export function sliderMarks(
  hints: SourceEditorHints,
  min: number,
  max: number,
  named: readonly NamedValue[],
  fallback: string | undefined,
  width: number
): { readonly marks: readonly SliderMark[]; readonly labels: readonly SliderLabel[] } {
  if (!(max > min)) {
    return { marks: [], labels: [] };
  }
  const at = (value: number): number => (value - min) / (max - min);
  const inRange = (value: number): boolean => Number.isFinite(value) && value >= min && value <= max;
  const stops = [...new Set(hints.stops ?? [])].filter(inRange).sort((a, b) => a - b);
  const references = new Map<number, string[]>();
  for (const value of named) {
    const text = value.value?.trim();
    if (text === undefined || !isNumericLiteral(text) || !inRange(Number(text))) {
      continue;
    }
    const number = Number(text);
    references.set(number, [...(references.get(number) ?? []), value.path]);
  }
  const marks: SliderMark[] = stops.filter((stop) => !references.has(stop)).map((stop) => ({ at: at(stop), value: stop, kind: "stop" }));
  for (const [value, paths] of references) {
    marks.push({ at: at(value), value, kind: "named", references: paths });
  }
  const start = fallback?.trim();
  if (start !== undefined && isNumericLiteral(start) && inRange(Number(start))) {
    marks.push({ at: at(Number(start)), value: Number(start), kind: "default" });
  }
  marks.sort((a, b) => a.at - b.at);

  // The first stop anchors the scale, so it is placed first, then the named values, the last stop
  // and the other stops.
  const number = (stop: number): SliderLabel => ({ at: at(stop), value: stop, text: String(stop) });
  const unnamed = stops.filter((stop) => !references.has(stop));
  const first = stops.length > 0 && unnamed.includes(stops[0]!) ? [stops[0]!] : [];
  const last = stops.length > 1 && unnamed.includes(stops[stops.length - 1]!) ? [stops[stops.length - 1]!] : [];
  const inner = stops.length <= LABELED_STOPS ? unnamed.filter((stop) => !first.includes(stop) && !last.includes(stop)) : [];
  const candidates: SliderLabel[] = [
    ...first.map(number),
    ...[...references].map(([value, paths]) => ({ at: at(value), value, text: paths[0]!, reference: paths[0]! })),
    ...last.map(number),
    ...inner.map(number),
  ];
  const placed: { readonly from: number; readonly to: number }[] = [];
  const labels: SliderLabel[] = [];
  for (const label of candidates) {
    const half = (label.text.length * LABEL_CHARACTER) / 2 + LABEL_GAP / 2;
    const middle = label.at * width;
    const span = { from: middle - half, to: middle + half };
    if (placed.some((other) => span.from < other.to && other.from < span.to)) {
      continue;
    }
    placed.push(span);
    labels.push(label);
  }
  labels.sort((a, b) => a.at - b.at);
  return { marks, labels };
}

/** Shows the named value `path` in place of what `span` draws, muted as waiting for the host. */
export function showReference(span: HTMLElement, path: string): void {
  span.classList.add("pending");
  span.replaceChildren(path);
}

/**
 * The options listing named values: each one's path, its color as a swatch when its value is a
 * color literal, its type and its doc's first paragraph, whose whole doc a card shows; `current`,
 * the value's own reference, marked.
 */
export function namedOptions(document: Document, values: readonly NamedValue[], current: string | undefined): HTMLElement[] {
  return values.map((named) => {
    const row = document.createElement("div");
    row.className = named.path === current ? "option named current" : "option named";
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(named.path === current));
    row.dataset["reference"] = named.path;
    row.dataset["name"] = named.path;
    const name = document.createElement("span");
    name.className = "option-name";
    const color = colorOf(document, named.value);
    if (color !== undefined) {
      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.style.backgroundColor = color;
      swatch.setAttribute("aria-hidden", "true");
      name.append(swatch);
    }
    name.append(named.path);
    const type = document.createElement("span");
    type.className = "option-type";
    type.textContent = named.type;
    name.append(" ", type);
    row.append(name);
    const words = firstParagraph(named.doc);
    if (words !== "") {
      const doc = document.createElement("span");
      doc.className = "option-doc";
      doc.textContent = words;
      row.append(doc);
    }
    return row;
  });
}

/** The color a literal names, as `"#c8a27a"` does, or nothing when it names none. */
function colorOf(document: Document, literal: string | undefined): string | undefined {
  if (literal === undefined || !literal.startsWith('"') || !literal.endsWith('"') || literal.length < 2) {
    return undefined;
  }
  const probe = document.createElement("span");
  probe.style.backgroundColor = literal.slice(1, -1).trim();
  return probe.style.backgroundColor === "" ? undefined : probe.style.backgroundColor;
}

/**
 * What an unwritten property's editor opens on: its default, when it is a literal its editor takes,
 * as a string's content or a case's name; otherwise nothing.
 */
export function defaultValue(written: string | undefined, editor: TypeEditor | undefined): string {
  const text = written?.trim();
  if (text === undefined || editor === undefined) {
    return "";
  }
  switch (editor.kind) {
    case "number":
      return isNumericLiteral(text) ? text : "";
    case "boolean":
      return text === "true" || text === "false" ? text : "";
    case "case": {
      const name = text.slice(text.lastIndexOf(".") + 1);
      return editor.cases?.some((candidate) => candidate.name === name) === true ? name : "";
    }
    default:
      return text.startsWith('"') && text.endsWith('"') && text.length >= 2 ? (decodeString(text) ?? "") : "";
  }
}

/**
 * An unwritten property's default as the reading reads values: a string without its quotes, with
 * its swatch when it is a color, a boolean as its mark, and anything else as written. Nothing for a
 * property with no default.
 */
export function defaultShown(
  document: Document,
  written: string | undefined,
  editor: TypeEditor | undefined,
  format: SourceValueFormat | undefined
): Node[] {
  const text = written?.trim();
  if (text === undefined) {
    return [];
  }
  if (text === "true" || text === "false") {
    const mark = document.createElement("span");
    mark.className = `sheet-boolean ${text}`;
    mark.textContent = BOOLEAN_MARKS[text];
    mark.title = text;
    mark.setAttribute("aria-label", text);
    return [mark];
  }
  if (text.startsWith('"') && text.endsWith('"') && text.length >= 2) {
    const words = decodeString(text) ?? text.slice(1, -1);
    if (editor?.kind === "color" || format === "color") {
      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.style.backgroundColor = words.trim();
      swatch.setAttribute("aria-hidden", "true");
      return swatch.style.backgroundColor === "" ? [document.createTextNode(words)] : [swatch, document.createTextNode(words)];
    }
    return [document.createTextNode(words)];
  }
  return [document.createTextNode(text)];
}

/** A doc comment's first paragraph as plain text, for one line under a case's name. */
export function firstParagraph(doc: string | undefined): string {
  if (doc === undefined) {
    return "";
  }
  const paragraph = doc.trim().split(/\n\s*\n/)[0] ?? "";
  return paragraph.replace(/[`*_]/g, "").replace(/\s+/g, " ").trim();
}

/** The options of a case's popover: each case's name, its doc's first paragraph, the current marked. */
export function caseOptions(document: Document, cases: readonly SourceCase[], current: string): HTMLElement[] {
  return cases.map((option) => {
    const row = document.createElement("div");
    row.className = option.name === current ? "option current" : "option";
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(option.name === current));
    row.dataset["name"] = option.name;
    const name = document.createElement("span");
    name.className = "option-name";
    name.textContent = option.name;
    row.append(name);
    const words = firstParagraph(option.doc);
    if (words !== "") {
      const doc = document.createElement("span");
      doc.className = "option-doc";
      doc.textContent = words;
      row.append(doc);
    }
    return row;
  });
}

/**
 * The color `text` names as `#rrggbb`, for the browser's color picker: a hex color as it is, less
 * any alpha, or the color `fallback` is drawn in, or black.
 */
export function pickerColor(text: string, fallback: Element | undefined): string {
  const hex = text.trim();
  if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(hex)) {
    return hex.slice(0, 7).toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3,4}$/.test(hex)) {
    return `#${[...hex.slice(1, 4)].map((digit) => digit + digit).join("")}`.toLowerCase();
  }
  const drawn = fallback === undefined ? "" : getComputedStyle(fallback).backgroundColor;
  const rgb = /^rgba?\((\d+)[, ]+(\d+)[, ]+(\d+)/.exec(drawn);
  if (rgb !== null) {
    return `#${rgb.slice(1, 4).map((part) => Number(part).toString(16).padStart(2, "0")).join("")}`;
  }
  return "#000000";
}

/**
 * The offset in `element`'s text nearest the point `x`, `y`, as a caret placed there by a click
 * would fall: on the line the point is on, before or after the character it is over. Measured from
 * the characters' boxes, since the browsers' own lookups do not reach into a shadow root alike.
 * The end of the text when nothing is laid out.
 */
export function offsetAtPoint(element: HTMLElement, x: number, y: number): number {
  const text = element.textContent ?? "";
  const node = element.firstChild;
  if (node === null || node.nodeType !== 3 || text === "") {
    return text.length;
  }
  const range = element.ownerDocument.createRange();
  let best = text.length;
  let bestLine = Infinity;
  let bestX = Infinity;
  for (let at = 0; at < text.length; at++) {
    range.setStart(node, at);
    range.setEnd(node, at + 1);
    const box = range.getBoundingClientRect?.();
    if (box === undefined || (box.width === 0 && box.height === 0)) {
      continue;
    }
    const line = y < box.top ? box.top - y : y > box.bottom ? y - box.bottom : 0;
    const across = x < box.left ? box.left - x : x > box.right ? x - box.right : 0;
    if (line < bestLine || (line === bestLine && across < bestX)) {
      bestLine = line;
      bestX = across;
      best = x < (box.left + box.right) / 2 ? at : at + 1;
    }
  }
  return best;
}
