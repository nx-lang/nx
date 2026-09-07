import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  NxDisposedResourceError,
  NxEvaluationError,
  NxLanguageSnapshot,
  NxLibraryRegistry,
  type CompletionList,
  type DiagnosticReport,
  type DocumentSymbol,
  type Hover
} from "../src/index.js";

const FORM = "nx://tenant/form.nx";

const PANEL_SOURCE = [
  "type Mode = light | dark",
  "let <Panel mode:Mode title:string /> = <div />",
  '<Panel mode=light title="Hello" />',
  ""
].join("\n");

/** The UTF-16 position of `marker` in `source`, the way an editor would report it. */
function positionOf(source: string, marker: string): { line: number; character: number } {
  const offset = source.indexOf(marker);
  if (offset < 0) {
    throw new Error(`marker ${marker} not in source`);
  }
  const before = source.slice(0, offset);
  const line = before.split("\n").length - 1;
  const character = before.length - (before.lastIndexOf("\n") + 1);
  return { line, character };
}

describe("NxLanguageSnapshot", () => {
  it("builds from logical URIs without touching the filesystem and answers by URI", () => {
    const snapshot = new NxLanguageSnapshot([
      { uri: FORM, source: PANEL_SOURCE, version: 4 },
      { uri: "nx://tenant/other.nx", source: "let other = 1\n" }
    ]);
    try {
      const symbols = snapshot.documentSymbols(FORM);
      expect(symbols.map((symbol) => symbol.name)).toContain("Panel");
      expect(snapshot.documentSymbols("nx://tenant/other.nx").map((symbol) => symbol.name)).toEqual(["other"]);
    } finally {
      snapshot.dispose();
    }
  });

  it("returns typed hover content with UTF-16 positions and byte offsets, and null without metadata", () => {
    const source = 'let greet(name:string) = { "😀" + name }\n';
    const snapshot = new NxLanguageSnapshot([{ uri: FORM, source, version: 2 }]);
    try {
      const hover = snapshot.hover(FORM, positionOf(source, "name }"));
      expect(hover).not.toBeNull();
      const { range, contents, version, identity, uri } = hover as Hover;
      expect(uri).toBe(FORM);
      expect(identity).toBe("tenant/form.nx");
      expect(version).toBe(2);
      expect(contents).toContain("string");
      // `name` starts at UTF-16 column 34; the emoji before it is four bytes, so byte 36.
      expect(range.start).toEqual({ line: 0, character: 34 });
      expect(range.end).toEqual({ line: 0, character: 38 });
      expect(range.startByte).toBe(36);
      expect(range.endByte).toBe(40);

      expect(snapshot.hover(FORM, { line: 0, character: 0 })).toBeNull();
    } finally {
      snapshot.dispose();
    }
  });

  it("returns typed completion items", () => {
    const source = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />\n<Panel  />\n";
    const snapshot = new NxLanguageSnapshot([{ uri: FORM, source, version: 1 }]);
    try {
      const completions: CompletionList = snapshot.completions(FORM, { line: 2, character: 7 });
      expect(completions.uri).toBe(FORM);
      expect(completions.version).toBe(1);
      const byLabel = new Map(completions.items.map((item) => [item.label, item]));
      expect(byLabel.get("mode")?.kind).toBe("Property");
      expect(byLabel.get("title")?.kind).toBe("Property");
      for (const item of completions.items) {
        expect(typeof item.label).toBe("string");
        expect(typeof item.kind).toBe("string");
        expect(item.detail === null || typeof item.detail === "string").toBe(true);
      }
    } finally {
      snapshot.dispose();
    }
  });

  it("reports diagnostics per document and workspace diagnostics separately", () => {
    const snapshot = new NxLanguageSnapshot([
      { uri: FORM, source: "let count: string = 1\n", version: 9 },
      { uri: "nx://tenant/ok.nx", source: "let fine = 1\n", version: 3 }
    ]);
    try {
      const report: DiagnosticReport = snapshot.diagnostics();
      expect(report.documents.map((document) => document.uri).sort()).toEqual([FORM, "nx://tenant/ok.nx"]);
      const form = report.documents.find((document) => document.uri === FORM);
      expect(form?.version).toBe(9);
      expect(form?.diagnostics.length).toBeGreaterThan(0);
      expect(form?.diagnostics[0]?.severity).toBe("Error");
      expect(form?.diagnostics[0]?.range.start.line).toBe(0);
      const ok = report.documents.find((document) => document.uri === "nx://tenant/ok.nx");
      expect(ok?.diagnostics).toEqual([]);
      expect(Array.isArray(report.workspace)).toBe(true);
    } finally {
      snapshot.dispose();
    }
  });

  it("sees library components through a build context, and not without one", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "nx-language-snapshot-"));
    const uiDir = join(tempRoot, "ui");
    mkdirSync(uiDir, { recursive: true });
    writeFileSync(
      join(uiDir, "button.nx"),
      "export type Size = small | large\nexport let <Button label:string size:Size /> = <button />\n"
    );
    const source = 'import { Button } from "../ui"\n<Button label="go" size=small />\n';
    const registry = new NxLibraryRegistry();
    registry.loadFromDirectory(uiDir);
    const buildContext = registry.createBuildContext();
    const withLibrary = new NxLanguageSnapshot([{ uri: FORM, source }], { buildContext });
    const withoutLibrary = new NxLanguageSnapshot([{ uri: FORM, source }]);
    try {
      const hover = withLibrary.hover(FORM, positionOf(source, "Button label"));
      expect(hover?.contents).toContain("<Button");
      expect(hover?.contents).toContain("size");
      expect(withLibrary.diagnostics().documents[0]?.diagnostics).toEqual([]);

      const blind = withoutLibrary.hover(FORM, positionOf(source, "Button label"));
      expect(blind?.contents ?? "").not.toContain("size");
      const messages = withoutLibrary
        .diagnostics()
        .documents.flatMap((document) => document.diagnostics)
        .map((diagnostic) => diagnostic.message);
      expect(messages.some((message) => message.includes("Missing workspace module or loaded library"))).toBe(true);
    } finally {
      withLibrary.dispose();
      withoutLibrary.dispose();
      buildContext.dispose();
      registry.dispose();
      rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  it("reports invalid input as an SDK error naming the offender", () => {
    expect(() => new NxLanguageSnapshot([{ uri: "not a uri", source: "" }])).toThrowError(NxEvaluationError);
    expect(() => new NxLanguageSnapshot([{ uri: "not a uri", source: "" }])).toThrowError(/not a uri/);

    expect(
      () =>
        new NxLanguageSnapshot([
          { uri: "nx://tenant/a.nx", source: "", identity: "tenant/same.nx" },
          { uri: "nx://tenant/b.nx", source: "", identity: "tenant/same.nx" }
        ])
    ).toThrowError(/tenant\/same\.nx/);

    const snapshot = new NxLanguageSnapshot([{ uri: FORM, source: "" }]);
    try {
      expect(() => snapshot.hover("nx://tenant/missing.nx", { line: 0, character: 0 })).toThrowError(
        /nx:\/\/tenant\/missing\.nx/
      );
    } finally {
      snapshot.dispose();
    }
  });

  it("throws the disposed-resource error after dispose, and disposes twice quietly", () => {
    const snapshot = new NxLanguageSnapshot([{ uri: FORM, source: "let value = 1\n" }]);
    snapshot.dispose();
    snapshot.dispose();
    expect(() => snapshot.diagnostics()).toThrowError(NxDisposedResourceError);
    expect(() => snapshot.hover(FORM, { line: 0, character: 0 })).toThrowError(NxDisposedResourceError);
  });
});

describe("protocol parity", () => {
  // The exact key sets the `@nx-lang/language-protocol` types declare, written down so that a serde
  // rename in the Rust service fails here rather than in a consumer. Keep in step with
  // packages/language-protocol/src/index.ts.
  const protocolKeys = {
    hover: ["contents", "identity", "range", "uri", "version"],
    range: ["end", "endByte", "start", "startByte"],
    position: ["character", "line"],
    completionList: ["identity", "items", "uri", "version"],
    completionItem: ["detail", "kind", "label"],
    diagnosticReport: ["documents", "workspace"],
    documentDiagnostics: ["diagnostics", "identity", "uri", "version"],
    editorDiagnostic: ["code", "message", "range", "related", "severity"],
    documentSymbol: ["kind", "name", "range", "selectionRange"]
  } as const;

  const sortedKeys = (value: object): string[] => Object.keys(value).sort();

  it("serializes every result kind with exactly the protocol's keys", () => {
    const source = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />\n<Panel  />\nlet bad: string = 1\n";
    const snapshot = new NxLanguageSnapshot([{ uri: FORM, source, version: 1 }]);
    try {
      const hover = snapshot.hover(FORM, { line: 1, character: 6 }) as Hover;
      expect(sortedKeys(hover)).toEqual([...protocolKeys.hover]);
      expect(sortedKeys(hover.range)).toEqual([...protocolKeys.range]);
      expect(sortedKeys(hover.range.start)).toEqual([...protocolKeys.position]);

      const completions = snapshot.completions(FORM, { line: 2, character: 7 });
      expect(sortedKeys(completions)).toEqual([...protocolKeys.completionList]);
      expect(completions.items.length).toBeGreaterThan(0);
      expect(sortedKeys(completions.items[0] as object)).toEqual([...protocolKeys.completionItem]);

      const report = snapshot.diagnostics();
      expect(sortedKeys(report)).toEqual([...protocolKeys.diagnosticReport]);
      expect(sortedKeys(report.documents[0] as object)).toEqual([...protocolKeys.documentDiagnostics]);
      const diagnostic = report.documents[0]?.diagnostics[0];
      expect(diagnostic).toBeDefined();
      expect(sortedKeys(diagnostic as object)).toEqual([...protocolKeys.editorDiagnostic]);

      const symbols: DocumentSymbol[] = snapshot.documentSymbols(FORM);
      expect(symbols.length).toBeGreaterThan(0);
      expect(sortedKeys(symbols[0] as object)).toEqual([...protocolKeys.documentSymbol]);
    } finally {
      snapshot.dispose();
    }
  });
});
