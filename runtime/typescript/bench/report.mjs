/** How the harness prints: times a person can read, and a table as text or as Markdown. */

/** A time in milliseconds, in the unit that leaves it three figures. */
export function formatTime(milliseconds) {
  if (milliseconds === undefined) {
    return "";
  }
  if (milliseconds < 0.001) {
    return `${(milliseconds * 1e6).toPrecision(3)} ns`;
  }
  return milliseconds < 1 ? `${(milliseconds * 1e3).toPrecision(3)} µs` : `${milliseconds.toPrecision(3)} ms`;
}

/** `rows` under `headings`, as columns padded with spaces: text is left-aligned, a column named in `right` right-aligned. */
export function textTable(headings, rows, right = []) {
  const widths = headings.map((heading, column) => Math.max(heading.length, ...rows.map((row) => row[column].length)));
  const line = (cells) =>
    cells
      .map((cell, column) => (right.includes(headings[column]) ? cell.padStart(widths[column]) : cell.padEnd(widths[column])))
      .join("  ")
      .trimEnd();
  return [line(headings), line(widths.map((width) => "-".repeat(width))), ...rows.map(line)].join("\n");
}

/** The same table as GitHub renders one. */
export function markdownTable(headings, rows, right = []) {
  const line = (cells) => `| ${cells.join(" | ")} |`;
  const rule = headings.map((heading) => (right.includes(heading) ? "---:" : "---"));
  return [line(headings), line(rule), ...rows.map(line)].join("\n");
}
