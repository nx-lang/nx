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

test("nested Color crosses WASM images, typed JSON input, and numerical schema with defaults", () => {
  const artifact = host.buildProgramArtifact(source, { fileName: "main.nx" });
  try {
    const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image => [image.identity, prepareNxIrModule(image.bytes)]));
    const program = linkNxIrProgram(modules.get("main.nx")!, { resolve: identity => modules.get(identity) });
    const original = evaluateFunction(program, "root", []);
    expect(evaluateFunction(program, "echo", [JSON.parse(JSON.stringify(original))])).toEqual(original);
    const color = evaluateFunction(program, "validateBrand", [{ accent: { r: 0.1, g: 0.2, b: 0.3 } }]);
    expect(color).toMatchObject({ $type: "Color", r: Math.fround(0.1), g: Math.fround(0.2), b: Math.fround(0.3), a: 1 });
    expect(() => evaluateFunction(program, "validateBrand", [{ accent: { r: 2, g: 0, b: 0 } }])).toThrow();
    const schema = artifact.functionSchema({ name: "echo" }).inputSchema!;
    const definitions = schema.$defs as Record<string, { properties?: Record<string, { type?: string; default?: unknown; minimum?: number }>; required?: string[] }>;
    const definition = Object.values(definitions).find(definition => definition.properties?.r?.type === "number")!;
    expect(definition).toBeDefined();
    expect(definition.properties!.a!.default).toBe(1);
    expect(definition.required).not.toContain("a");
    for (const name of ["r", "g", "b", "a"]) expect(definition.properties![name]!.type).toBe("number");
    expect(definition.properties!.r!.minimum).toBeUndefined();
  } finally { artifact.dispose(); }
});
