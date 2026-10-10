import { describe, expect, it } from "vitest";

import { createNxLanguageHandler } from "@nx-lang/language-http";
import type { DiagnosticReport, Hover, SourceTree } from "@nx-lang/language-protocol";

import { createLanguageService } from "../src/language.js";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

const uri = "nx://tenant/form.nx";
const catalogUri = "nx://host/catalog.nx";
const catalog =
  "export type Mode = light | dark\nexport let <Panel mode:Mode title:string /> = <div />";
const source = '<Panel mode=light title="x" />\n';
const documents = [{ uri, source, version: 5 }];

const host = createNxHost(nxModule);
const service = createLanguageService(host, {
  documents: [{ uri: catalogUri, identity: "catalog.nx", source: catalog }],
  implicitImports: ["catalog.nx"]
});
// The HTTP handler over the same mechanism — a context document named as an implicit import — so the
// two transports are compared over one feature rather than two.
const handler = createNxLanguageHandler({
  context: {
    documents: [{ uri: catalogUri, identity: "catalog.nx", source: catalog }],
    implicitImports: ["catalog.nx"]
  }
});

async function overHttp<T>(query: string, body: unknown): Promise<T> {
  const response = await handler(
    new Request(`http://localhost/api/language/${query}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body)
    })
  );
  expect(response.status).toBe(200);
  return (await response.json()) as T;
}

describe("the wasm SDK's in-process language service", () => {
  it("answers hover through an implicit import in the document's own coordinates", async () => {
    const position = { line: 0, character: 3 };
    const hover = await service.hover({ documents, uri, position });

    expect(hover).not.toBeNull();
    expect(hover!.contents).toContain("<Panel");
    expect(hover!.range.start).toEqual({ line: 0, character: 1 });
    expect(hover!.version).toBe(5);

    // The same query through the HTTP handler, which reaches the declaration the same way.
    expect(hover).toEqual(await overHttp<Hover | null>("hover", { documents, uri, position }));
  });

  it("offers an implicitly imported component's properties inside its opening tag", async () => {
    const position = { line: 0, character: 7 };
    const completions = await service.completions({ documents, uri, position });

    // `title` is already written in the tag, so what is offered is the property still missing.
    const labels = completions.items.map((item) => item.label);
    expect(labels).toContain("mode");
    expect(completions).toEqual(await overHttp("completions", { documents, uri, position }));
  });

  it("answers document symbols identically to the HTTP handler", async () => {
    expect(await service.documentSymbols({ documents, uri })).toEqual(
      await overHttp("documentSymbols", { documents, uri })
    );
  });

  // Spec: "Parity between Rust and TypeScript". The HTTP handler answers through the Node SDK, the
  // Rust language service's own serialization; this service answers through the wasm module.
  it("answers the source tree identically to the HTTP handler, for every document of the corpus", async () => {
    const corpus = [
      { uri, source, version: 5 },
      // A comment, and a line that does not parse.
      { uri, source: `${source}// a comment\nlet broken = %%%\n`, version: 6 },
      {
        uri,
        source: [
          "type Card = { title:string content body?:string = \"x\" }",
          "let card = <Card title={\"😀\" + \"!\"}>Hello</Card>",
          "let pick(m:Mode) = { if m is { Mode.light => 1 else => 2 } }",
          "let xs = { 1 2 }",
          "let total = { for x, i in xs { x + i } }",
          ""
        ].join("\n"),
        version: 7
      }
    ];
    for (const document of corpus) {
      const request = { documents: [document], uri };
      const fromWasm = await service.sourceTree(request);
      const fromRust = await overHttp<SourceTree>("sourceTree", request);
      expect(JSON.parse(JSON.stringify(fromWasm))).toEqual(fromRust);
      expect(fromWasm.version).toBe(document.version);
      expect(fromWasm.nodes.length).toBeGreaterThan(0);
    }
  });

  it("reports a fault in a host document against that document's URI", async () => {
    const brokenService = createLanguageService(host, {
      documents: [
        { uri: catalogUri, identity: "catalog.nx", source: `${catalog}\nlet wrong: string = 1` }
      ],
      implicitImports: ["catalog.nx"]
    });

    const report: DiagnosticReport = await brokenService.diagnostics({ documents, uri });

    const queried = report.documents.find((document) => document.uri === uri);
    expect(queried?.diagnostics ?? []).toEqual([]);
    const hostDocument = report.documents.find((document) => document.uri === catalogUri);
    expect(hostDocument).toBeDefined();
    expect(hostDocument!.diagnostics.length).toBeGreaterThan(0);
    expect(hostDocument!.diagnostics[0]!.range.start.line).toBe(2);
  });

  it("analyzes a repeated document set once and echoes each request's version", async () => {
    const first = await service.hover({ documents, uri, position: { line: 0, character: 3 } });
    const second = await service.hover({
      documents: [{ uri, source, version: 6 }],
      uri,
      position: { line: 0, character: 3 }
    });

    expect(first!.version).toBe(5);
    expect(second!.version).toBe(6);
  });
});
