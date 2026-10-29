import { afterAll, expect, test } from "vitest";
import {
  evaluateFunction,
  linkNxIrProgram,
  prepareNxIrModule,
  type NxRuntimeOrigins,
} from "@nx-lang/ir-runtime";
import { createNxHost } from "../src/node.js";
import type {
  NxGraphicsConfig,
  NxGraphicsPreparationRequest,
} from "../src/graphics.js";
import { nxModule } from "./support.js";
const host = createNxHost(nxModule);
afterAll(() => host.dispose());
const config: NxGraphicsConfig = {
  width: 100,
  height: 100,
  scale: 1,
  program_revision: "",
  graphics_version: "",
  renderer: "sdk-test",
  fonts: [],
  limits: {
    nodes: 100,
    depth: 20,
    path_commands: 10000,
    coordinate: 1e7,
    surface_pixels: 10000,
    layer_pixels: 40000,
  },
};
function requestFor(source: string) {
  const artifact = host.buildProgramArtifact(source, { fileName: "main.nx" });
  const modules = new Map(
    artifact
      .generateNxIr({ debug: true, modules: [] })
      .map((image) => [image.identity, prepareNxIrModule(image.bytes)]),
  );
  const program = linkNxIrProgram(modules.get("main.nx")!, {
    resolve: (identity) => modules.get(identity),
  });
  const origins: NxRuntimeOrigins = { includeCallSites: true };
  const value = evaluateFunction(program, "root", [], { origins });
  return {
    artifact,
    request: {
      value,
      origins: origins.entries!,
      config,
    } satisfies NxGraphicsPreparationRequest,
  };
}
const prefix =
  'import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>>';
test("shared graphics preparation resolves genuine aliases and rejects same-name spoofs", () => {
  for (const [source, valid] of [
    [prefix + "<g.Rect width=10 height=20/></g.Drawing>", true],
    [
      'import "@nx/graphics" as g\nexternal component <Rect width:float64 height:float64/>\nlet root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>><Rect width=10 height=20/></g.Drawing>',
      false,
    ],
  ] as const) {
    const { artifact, request } = requestFor(source);
    try {
      const result = artifact.prepareGraphics(request);
      expect(result.scene !== null).toBe(valid);
      if (valid)
        expect(result.scene!.root.children[0]!.identity.module).toBe(
          "@nx/graphics/graphics.nx",
        );
      else expect(result.diagnostics[0]!.message).toMatch(/identity/);
    } finally {
      artifact.dispose();
    }
  }
});
test("preflight returns validated text requests; actual preparation requires measurements", () => {
  const { artifact, request } = requestFor(
    prefix + '<g.Text fontFamily="Fixture">hello</g.Text></g.Drawing>',
  );
  try {
    const result = artifact.prepareGraphics({ ...request, preflight: true });
    expect(result.scene).toBeNull();
    expect(result.diagnostics).toEqual([]);
    expect(result.textRequests).toHaveLength(1);
    expect(result.textRequests![0]!.request.text).toBe("hello");
    expect(result.textRequests![0]!.element).toMatch(/children/);
    expect(result.textRequests![0]!.provenance!.module).toBe("main.nx");
    expect(artifact.prepareGraphics(request).diagnostics[0]!.message).toMatch(
      /measurement/,
    );
    expect(() =>
      artifact.prepareGraphics({} as NxGraphicsPreparationRequest),
    ).toThrow();
    expect(() =>
      artifact.prepareGraphics({
        ...request,
        config: { ...config, width: Infinity },
      }),
    ).toThrow(/finite/);
  } finally {
    artifact.dispose();
  }
  expect(() => artifact.prepareGraphics(request)).toThrow(/disposed/i);
});
