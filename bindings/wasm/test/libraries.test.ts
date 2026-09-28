import {
  evaluateFunction,
  linkNxIrProgram,
  prepareNxIrModule,
  type NxPreparedModule
} from "@nx-lang/ir-runtime";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { NxDisposedResourceError, NxEvaluationError, NxWasmError } from "../src/errors.js";
import type { NxHost, NxLibraryRegistry, NxProgramBuildContext } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import type { NxGeneratedNxIr, NxLibraryInput } from "../src/types.js";
import { nxModule } from "./support.js";

const questionFlow: NxLibraryInput = {
  root: "libraries/question-flow",
  version: "3",
  modules: [
    { identity: "Step.nx", source: "export type Step = { id:string }" },
    { identity: "QuestionFlow.nx", source: "export type QuestionFlow = { firstStep:Step }" }
  ]
};

const chatLink: NxLibraryInput = {
  root: "libraries/chat-link",
  modules: [
    {
      identity: "ChatLinkConfig.nx",
      source:
        'import "../question-flow"\nexport type ChatLinkConfig = { title:string questionFlow:QuestionFlow }'
    }
  ]
};

const implicitImports = ["libraries/chat-link", "libraries/question-flow"];

function tenant(title: string): string {
  return `let root() = <ChatLinkConfig title="${title}" questionFlow={<QuestionFlow firstStep={<Step id="a" />} />} />`;
}

function codeOf(error: unknown): readonly (string | undefined)[] {
  expect(error).toBeInstanceOf(NxEvaluationError);
  return (error as NxEvaluationError).diagnostics.map((diagnostic) => diagnostic.code);
}

function thrown(run: () => unknown): unknown {
  try {
    run();
  } catch (error) {
    return error;
  }
  throw new Error("expected the call to throw");
}

describe("in-memory libraries", () => {
  let host: NxHost;
  let registry: NxLibraryRegistry;
  let context: NxProgramBuildContext;

  beforeAll(() => {
    host = createNxHost(nxModule);
    registry = host.createLibraryRegistry();
    // chat-link imports question-flow; the registry loads them in that order whatever the list says.
    registry.loadLibraries([chatLink, questionFlow]);
    context = registry.createBuildContext({ implicitImports });
  });

  afterAll(() => {
    context.dispose();
    registry.dispose();
    host.dispose();
  });

  function emitEveryModule(source: string): readonly NxGeneratedNxIr[] {
    const artifact = host.buildWorkspaceArtifact({
      modules: [{ identity: "chat-link.nx", source }],
      entry: "chat-link.nx",
      buildContext: context
    });
    try {
      return artifact.generateNxIr({ modules: [] });
    } finally {
      artifact.dispose();
    }
  }

  it("builds a tenant that names the libraries' declarations with no import line", () => {
    const images = emitEveryModule(tenant("Hi"));
    expect(images.map((image) => image.identity)).toEqual([
      "chat-link.nx",
      "libraries/chat-link/ChatLinkConfig.nx",
      "libraries/question-flow/QuestionFlow.nx",
      "libraries/question-flow/Step.nx"
    ]);

    const entry = prepareNxIrModule(images[0]!.bytes).artifact;
    expect(entry.modules).toContainEqual(
      expect.objectContaining({ identity: "libraries/chat-link/ChatLinkConfig.nx", version: "" })
    );
    const step = prepareNxIrModule(images[3]!.bytes);
    expect(step.version).toBe("3");
  });

  it("emits each library module identically whichever tenant it was built with", () => {
    const libraryImages = (images: readonly NxGeneratedNxIr[]) =>
      images.filter((image) => image.identity.startsWith("libraries/")).map((image) => image.bytes);

    expect(libraryImages(emitEveryModule(tenant("One")))).toEqual(
      libraryImages(emitEveryModule(`${tenant("Two")}\nlet key() = {Step.Property.id}`))
    );
  });

  it("prepares the library images once and links two entries against them", () => {
    const first = emitEveryModule(tenant("First"));
    const second = emitEveryModule(tenant("Second"));

    let preparations = 0;
    const prepared = new Map<string, NxPreparedModule>();
    for (const image of first.filter((image) => image.identity.startsWith("libraries/"))) {
      prepared.set(image.identity, prepareNxIrModule(image.bytes));
      preparations += 1;
    }
    const resolve = (identity: string) => prepared.get(identity);

    for (const [images, title] of [
      [first, "First"],
      [second, "Second"]
    ] as const) {
      const program = linkNxIrProgram(prepareNxIrModule(images[0]!.bytes), { resolve });
      expect(evaluateFunction(program, "root")).toMatchObject({
        $type: "ChatLinkConfig",
        title,
        questionFlow: { $type: "QuestionFlow", firstStep: { $type: "Step", id: "a" } }
      });
    }
    expect(preparations).toBe(3);
  });

  it("validates a workspace against the libraries and answers with diagnostics as data", () => {
    expect(
      host.validateWorkspace({
        modules: [{ identity: "chat-link.nx", source: tenant("Hi") }],
        buildContext: context
      })
    ).toEqual([]);

    const diagnostics = host.validateWorkspace({
      modules: [{ identity: "chat-link.nx", source: "let root() = <ChatLinkConfig title={1} />" }],
      buildContext: context
    });
    expect(diagnostics.length).toBeGreaterThan(0);
    expect(diagnostics[0]!.severity).toBe("error");
    expect(diagnostics[0]!.labels[0]!.file).toBe("chat-link.nx");
    expect(diagnostics[0]!.labels[0]!.span.startLine).toBe(1);

    // Without the context, the names are not in scope and the implicit imports name nothing.
    const withoutLibraries = host.validateWorkspace({
      modules: [{ identity: "chat-link.nx", source: tenant("Hi") }],
      implicitImports
    });
    expect(withoutLibraries.map((diagnostic) => diagnostic.code)).toContain(
      "implicit-import-not-found"
    );
  });

  it("reports a missing dependency and a changed root, and keeps what was loaded", () => {
    const fresh = host.createLibraryRegistry();
    try {
      expect(codeOf(thrown(() => fresh.loadLibrary(chatLink)))).toContain(
        "library-dependency-missing"
      );
      fresh.loadLibrary(questionFlow);
      // An identical reload is a no-op; a changed one is refused.
      fresh.loadLibrary(questionFlow);
      expect(codeOf(thrown(() => fresh.loadLibrary({ ...questionFlow, version: "4" })))).toContain(
        "library-root-conflict"
      );
      fresh.loadLibrary(chatLink);
    } finally {
      fresh.dispose();
    }
  });

  it("refuses a cycle among the libraries it is given", () => {
    const fresh = host.createLibraryRegistry();
    try {
      const error = thrown(() =>
        fresh.loadLibraries([
          { root: "libraries/a", modules: [{ identity: "A.nx", source: 'import "../b"' }] },
          { root: "libraries/b", modules: [{ identity: "B.nx", source: 'import "../a"' }] }
        ])
      );
      expect(codeOf(error)).toContain("library-dependency-cycle");
    } finally {
      fresh.dispose();
    }
  });

  it("limits a context to the roots it names and their dependencies", () => {
    const flowOnly = registry.createBuildContext({ visibleRoots: ["libraries/question-flow"] });
    try {
      const artifact = host.buildWorkspaceArtifact({
        modules: [{ identity: "main.nx", source: 'let root() = <Step id="a" />' }],
        entry: "main.nx",
        implicitImports: ["libraries/question-flow"],
        buildContext: flowOnly
      });
      artifact.dispose();

      const diagnostics = host.validateWorkspace({
        modules: [{ identity: "chat-link.nx", source: tenant("Hi") }],
        implicitImports,
        buildContext: flowOnly
      });
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toContain(
        "implicit-import-not-found"
      );
    } finally {
      flowOnly.dispose();
    }
  });

  it("refuses a workspace module that lies inside a library's root", () => {
    for (const identity of ["libraries/question-flow/Step.nx", "libraries/question-flow/Evil.nx"]) {
      const modules = [
        { identity: "chat-link.nx", source: tenant("Hi") },
        { identity, source: "export type Step = { y:int }" }
      ];
      const diagnostics = host.validateWorkspace({ modules, buildContext: context });
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toContain(
        "workspace-module-in-library-root"
      );
      expect(
        codeOf(
          thrown(() =>
            host.buildWorkspaceArtifact({ modules, entry: "chat-link.nx", buildContext: context })
          )
        )
      ).toContain("workspace-module-in-library-root");
    }
  });

  it("answers a library's warnings at load and leaves them out of tenant validation", () => {
    const fresh = host.createLibraryRegistry();
    try {
      const warnings = fresh.loadLibrary({
        root: "libraries/warns",
        modules: [
          {
            identity: "Warns.nx",
            source: "export type Warns = { id:string }\nexport let f(n:int) = { n ?? 0 }"
          }
        ]
      });
      expect(warnings.map((diagnostic) => diagnostic.code)).toEqual(["fallback-never-taken"]);
      expect(warnings[0]!.labels[0]!.file).toBe("libraries/warns/Warns.nx");
      expect(warnings[0]!.labels[0]!.span.startLine).toBe(2);

      const warnsContext = fresh.createBuildContext({ implicitImports: ["libraries/warns"] });
      try {
        expect(
          host.validateWorkspace({
            modules: [{ identity: "tenant.nx", source: 'let root() = <Warns id="a" />' }],
            buildContext: warnsContext
          })
        ).toEqual([]);
      } finally {
        warnsContext.dispose();
      }
    } finally {
      fresh.dispose();
    }
  });

  it("throws a failed build with exactly the diagnostics validation answers with", () => {
    const options = {
      modules: [{ identity: "chat-link.nx", source: "let root() = <ChatLinkConfig title={1} />" }],
      buildContext: context
    };
    const validation = host.validateWorkspace(options);
    const error = thrown(() => host.buildWorkspaceArtifact({ ...options, entry: "chat-link.nx" }));
    expect(error).toBeInstanceOf(NxEvaluationError);
    expect((error as NxEvaluationError).diagnostics).toEqual(validation);
  });

  it("replaces the context's implicit imports with a list given per call, even an empty one", () => {
    // With the context's implicit imports, `Step` is the library's record and `id` must be a
    // string; with none, it is an element NX does not check.
    const modules = [{ identity: "main.nx", source: "let root() = <Step id={1} />" }];
    expect(host.validateWorkspace({ modules, buildContext: context }).length).toBeGreaterThan(0);
    expect(host.validateWorkspace({ modules, buildContext: context, implicitImports: [] })).toEqual(
      []
    );
  });

  it("names a visible root that is not a loaded library", () => {
    const error = thrown(() => registry.createBuildContext({ visibleRoots: ["libraries/nope"] }));
    expect(error).toBeInstanceOf(NxEvaluationError);
    expect((error as Error).message).toContain(
      "Visible root 'libraries/nope' is not a loaded library"
    );
  });

  it("follows the SDK's lifecycle rules for registries and contexts", () => {
    const fresh = host.createLibraryRegistry();
    fresh.loadLibrary(questionFlow);
    const kept = fresh.createBuildContext();
    fresh.dispose();
    fresh.dispose();

    expect(() => fresh.createBuildContext()).toThrowError(NxDisposedResourceError);
    expect(() => fresh.loadLibrary(questionFlow)).toThrowError(NxDisposedResourceError);

    // A context holds the libraries it sees, so it outlives its registry.
    const artifact = host.buildWorkspaceArtifact({
      modules: [{ identity: "main.nx", source: 'let root() = <Step id="a" />' }],
      entry: "main.nx",
      implicitImports: ["libraries/question-flow"],
      buildContext: kept
    });
    artifact.dispose();

    kept.dispose();
    kept.dispose();
    expect(() =>
      host.validateWorkspace({ modules: [], buildContext: kept })
    ).toThrowError(NxDisposedResourceError);

    const other = createNxHost(nxModule);
    try {
      expect(() => other.validateWorkspace({ modules: [], buildContext: context })).toThrowError(
        NxWasmError
      );
    } finally {
      other.dispose();
    }
  });
});
