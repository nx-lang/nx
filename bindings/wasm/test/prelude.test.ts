import { describe, expect, it } from "vitest";

import { createNxHost } from "../src/node.js";
import { buildProgramWithPrelude } from "../src/prelude.js";
import { nxModule } from "./support.js";

const host = createNxHost(nxModule);

// A prelude long enough that a document line and its combined-module line cannot be confused, and
// with a declaration the document can use.
const prelude = [
  "abstract component <View />",
  'external component <Label extends View Text:string = "" />',
  ...Array.from({ length: 600 }, (_, index) => `// filler ${index}`)
].join("\n");

function build(source: string, over = prelude) {
  return buildProgramWithPrelude(host, over, source, { fileName: "document.nx" });
}

describe("buildProgramWithPrelude", () => {
  it("yields IR and no diagnostics for a clean build", () => {
    const result = build('let root() = { <Label Text="hi" /> }');

    expect(result.diagnostics).toEqual([]);
    expect(result.ir).not.toBeNull();
    expect(JSON.parse(result.ir!.json)).toMatchObject({ format: "nx-ir-json" });
    expect(result.ir!.metadata.functionEntrypoints.map((entry) => entry.name)).toContain("root");
  });

  it("reports a document error in the document's own coordinates", () => {
    const result = build("let root() = {\n\n  <Label Text=1.0 />\n}");

    expect(result.ir).toBeNull();
    expect(result.diagnostics).toHaveLength(1);
    const [diagnostic] = result.diagnostics;
    expect(diagnostic!.origin).toBe("source");
    // Line 3, column 10 is the `T` of `Text`: the prelude's six hundred lines are subtracted.
    expect(diagnostic!.span).toMatchObject({ startLine: 3, startColumn: 10, endLine: 3 });
    expect(diagnostic!.span!.startByte).toBe("let root() = {\n\n  <Label ".length);
  });

  it("attributes a fault inside the prelude to the catalog, with no IR and no document span", () => {
    const result = build('let root() = { <Label Text="hi" /> }', `${prelude}\nexternal component <Broken\n`);

    expect(result.ir).toBeNull();
    expect(result.diagnostics.length).toBeGreaterThan(0);
    for (const diagnostic of result.diagnostics) {
      expect(diagnostic.origin).toBe("catalog");
      expect(diagnostic.span).toBeNull();
    }
  });

  it("reports a whole-program failure without a position", () => {
    const result = build(
      'let root() = { <Label Text="hi" /> }',
      `${prelude}\nexternal component <Broken value: NoSuchType? />\n`
    );

    expect(result.ir).toBeNull();
    expect(result.diagnostics.length).toBeGreaterThan(0);
    for (const diagnostic of result.diagnostics) {
      expect(diagnostic.origin).toBe("program");
      expect(diagnostic.span).toBeNull();
    }
  });

  it("keeps an insertion point, which the compiler reports as an empty span", () => {
    // `Expected } here` names a column and no width. That is a position the author can act on, so it
    // must survive as one rather than be mistaken for a whole-program fault.
    const result = build('let root() = { <Label Text="hi" />');

    expect(result.ir).toBeNull();
    expect(result.diagnostics).toHaveLength(1);
    const [diagnostic] = result.diagnostics;
    expect(diagnostic!.origin).toBe("source");
    expect(diagnostic!.span!.startLine).toBe(1);
    expect(diagnostic!.span!.startColumn).toBe(diagnostic!.span!.endColumn);
  });

  it("compiles a document that is a single trailing element", () => {
    const result = build('<Label Text="hi" />\n');

    expect(result.diagnostics).toEqual([]);
    expect(result.ir).not.toBeNull();
  });
});
