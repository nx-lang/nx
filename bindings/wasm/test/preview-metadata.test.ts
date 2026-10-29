import { afterAll, expect, test } from "vitest";
import { createNxHost } from "../src/node.js";
import { NxLanguageSnapshot as NativeSnapshot } from "@nx-lang/sdk-node";
import { nxModule } from "./support.js";
const host = createNxHost(nxModule);
afterAll(() => host.dispose());

test("loaded library preview metadata is discoverable even without a direct constructor reference", () => {
  const snapshot = host.createLanguageSnapshot([
    {
      uri: "nx://test/main.nx",
      identity: "main.nx",
      source: 'import "@nx/graphics" as g\nlet root() = 42',
    },
  ]);
  try {
    const plain = snapshot.sourceTree("nx://test/main.nx", { visualizers: {} });
    expect(plain.declarations.some((entry) => entry.name === "Drawing")).toBe(
      false,
    );
    const tree = snapshot.sourceTree("nx://test/main.nx", {
      visualizers: { includeUnreferencedDeclarations: true },
    });
    const drawing = tree.declarations.find(
      (entry) =>
        entry.module === "@nx/graphics/graphics.nx" && entry.name === "Drawing",
    );
    expect(drawing?.visualizer?.preview).toEqual({ renderer: "nx.graphics" });
    expect(drawing?.visualizer?.icon).toBe("frame");
    expect(
      tree.declarations.find((entry) => entry.name === "Star")?.visualizer
        ?.icon,
    ).toBe("star");
    expect(JSON.parse(JSON.stringify(tree))).toEqual(tree);
    const native = new NativeSnapshot([
      {
        uri: "nx://test/main.nx",
        identity: "main.nx",
        source: 'import "@nx/graphics" as g\nlet root() = 42',
      },
    ]);
    try {
      expect(JSON.parse(JSON.stringify(tree))).toEqual(
        native.sourceTree("nx://test/main.nx", {
          visualizers: { includeUnreferencedDeclarations: true },
        }),
      );
    } finally {
      native.dispose();
    }
  } finally {
    snapshot.dispose();
  }
});

test("an aliased visualizer descriptor resolves by declaration and keeps typed preview metadata", () => {
  const source =
    'import "@nx/visualizer" as v\ntype Question = { label:string }\nvisualizer Question = <v.ElementVisualizer icon="list" preview=<v.PreviewVisualizer renderer="app.survey"/>/>\n<Question label="Which?"/>';
  const artifact = host.buildProgramArtifact(source, { fileName: "main.nx" });
  const snapshot = host.createLanguageSnapshot([
    { uri: "nx://test/main.nx", identity: "main.nx", source },
  ]);
  try {
    expect(artifact.diagnostics()).toEqual([]);
    const tree = snapshot.sourceTree("nx://test/main.nx", { visualizers: {} });
    expect(
      tree.declarations.find((entry) => entry.name === "Question")?.visualizer
        ?.preview,
    ).toEqual({ renderer: "app.survey" });
  } finally {
    snapshot.dispose();
    artifact.dispose();
  }
});
