import { describe, expect, it } from "vitest";

import { createNxLanguageHandler } from "@nx-lang/language-http";
import type { DiagnosticReport, Hover } from "@nx-lang/language-protocol";

import { createLanguageService } from "../src/language.js";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

const uri = "nx://tenant/form.nx";
const panel = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />";
const source = '<Panel mode=light title="x" />\n';
const documents = [{ uri, source, version: 5 }];

const host = createNxHost(nxModule);
const service = createLanguageService(host, { prelude: { source: panel } });
const handler = createNxLanguageHandler({ prelude: { source: panel } });

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
  it("answers hover through a prelude in the document's own coordinates", async () => {
    const position = { line: 0, character: 3 };
    const hover = await service.hover({ documents, uri, position });

    expect(hover).not.toBeNull();
    expect(hover!.contents).toContain("<Panel");
    expect(hover!.range.start).toEqual({ line: 0, character: 1 });
    expect(hover!.version).toBe(5);

    // The same query through the HTTP handler, which runs the same core over the Node SDK.
    expect(hover).toEqual(await overHttp<Hover | null>("hover", { documents, uri, position }));
  });

  it("answers completions, diagnostics and document symbols identically to the HTTP handler", async () => {
    const position = { line: 0, character: 7 };

    expect(await service.completions({ documents, uri, position })).toEqual(
      await overHttp("completions", { documents, uri, position })
    );
    expect(await service.diagnostics({ documents, uri })).toEqual(
      await overHttp<DiagnosticReport>("diagnostics", { documents, uri })
    );
    expect(await service.documentSymbols({ documents, uri })).toEqual(
      await overHttp("documentSymbols", { documents, uri })
    );
  });

  it("reports a diagnostic inside the prelude with a prelude origin and no range, as the handler does", async () => {
    const brokenPrelude = `${panel}\nlet wrong: string = 1`;
    const brokenService = createLanguageService(host, { prelude: { source: brokenPrelude } });
    const brokenHandler = createNxLanguageHandler({ prelude: { source: brokenPrelude } });

    const fromWasm = await brokenService.diagnostics({ documents, uri });
    const response = await brokenHandler(
      new Request("http://localhost/api/language/diagnostics", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ documents, uri })
      })
    );
    const fromHttp = (await response.json()) as DiagnosticReport;

    expect(fromWasm.workspace.length).toBeGreaterThan(0);
    expect(fromWasm.workspace[0]!.labels[0]!.identity).toBe("prelude");
    expect(fromWasm).toEqual(fromHttp);
  });
});
