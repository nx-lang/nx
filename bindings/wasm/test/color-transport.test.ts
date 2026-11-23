import { afterAll, expect, test } from "vitest";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

const host = createNxHost(nxModule);
afterAll(() => host.dispose());
const source = `import "@nx/color" as c
export type Brand = { accent:c.Color }
let root() = <Brand accent=#1234/>
let echo(brand:Brand):Brand = { brand }
let validateBrand(brand:Brand):c.Color = { c.validate(brand.accent) }
`;

test("nested Color crosses WASM images, typed JSON input, and a schema with a definition per space", () => {
  const artifact = host.buildProgramArtifact(source, { fileName: "main.nx" });
  try {
    const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image => [image.identity, prepareNxIrModule(image.bytes)]));
    const program = linkNxIrProgram(modules.get("main.nx")!, { resolve: identity => modules.get(identity) });
    const original = evaluateFunction(program, "root", []);
    expect(evaluateFunction(program, "echo", [JSON.parse(JSON.stringify(original))])).toEqual(original);
    const color = evaluateFunction(program, "validateBrand", [{ accent: { $type: "Color.srgb", r: 0.1, g: 0.2, b: 0.3 } }]);
    expect(color).toMatchObject({ $type: "Color.srgb", r: Math.fround(0.1), g: Math.fround(0.2), b: Math.fround(0.3), alpha: 1 });
    expect(() => evaluateFunction(program, "validateBrand", [{ accent: { $type: "Color.srgb", r: 2, g: 0, b: 0 } }])).toThrow();
    const schema = artifact.functionSchema({ name: "echo" }).inputSchema!;
    const definitions = schema.$defs as Record<string, { anyOf?: { $ref: string }[]; properties?: Record<string, { type?: string; default?: unknown; minimum?: number; maximum?: number }>; required?: string[] }>;
    expect(definitions["Color"]!.anyOf!.map(option => option.$ref)).toEqual(
      ["srgb", "displayP3", "oklab", "oklch"].map(space => `#/$defs/Color.${space}`)
    );
    const definition = definitions["Color.srgb"]!;
    expect(definition.properties!.alpha!.default).toBe(1);
    expect(definition.required).not.toContain("alpha");
    for (const name of ["r", "g", "b", "alpha"]) {
      expect(definition.properties![name]!.type).toBe("number");
      expect(definition.properties![name]!.minimum).toBe(0);
      expect(definition.properties![name]!.maximum).toBe(1);
    }
  } finally { artifact.dispose(); }
});
