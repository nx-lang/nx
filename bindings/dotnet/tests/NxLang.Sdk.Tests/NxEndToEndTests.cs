// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using NxLang.Nx;
using NxLang.Nx.Interop;
using Xunit;

namespace NxLang.Nx.Tests;

public class NxEndToEndTests
{












    [Fact]
    public void NativeLibrary_IsStagedAlongsideTestOutput()
    {
        string nativeLibraryPath = Path.Combine(AppContext.BaseDirectory, NxNativeLibraryInfo.GetFileName());

        Assert.True(
            File.Exists(nativeLibraryPath),
            $"Expected the staged NX native runtime at '{nativeLibraryPath}'. Build `cargo build --release -p nx-ffi` before running dotnet tests.");
    }





    [Fact]
    public void GenerateJSProgramModule_WithProgramArtifact_ReturnsSourceAndMetadata()
    {
        using NxProgramArtifact artifact = NxProgramArtifact.Build(
            """let root() = { <div class="test" /> }""");

        NxGeneratedJSProgramModule module = artifact.GenerateJSProgramModule(
            new NxJSProgramModuleOptions
            {
                LogicalModuleName = "managed/main",
                RuntimeImportSpecifier = "./nx-runtime.js",
            });

        Assert.Equal("managed/main", module.LogicalModuleName);
        Assert.Equal("./nx-runtime.js", module.RuntimeImportSpecifier);
        Assert.Equal("nx-js-runtime-v1", module.RuntimeAbi);
        Assert.True(module.ProgramFingerprint > 0);
        NxGeneratedJSProgramModuleFunctionExport entrypoint = Assert.Single(module.FunctionExports);
        Assert.Equal("root", entrypoint.EntrypointName);
        Assert.Equal("root", entrypoint.ExportName);
        Assert.Empty(module.ComponentExports);
        Assert.Contains("import { nxElement } from \"./nx-runtime.js\";", module.SourceText);
        Assert.Contains("export function root()", module.SourceText);
        Assert.Contains("export const nxProgramModuleManifest", module.SourceText);
        Assert.DoesNotContain("export function nxElement", module.SourceText);
    }

    [Fact]
    public void GenerateJSProgramModule_WithCodegenDiagnostics_ThrowsEvaluationException()
    {
        using NxProgramArtifact artifact = NxProgramArtifact.Build(
            """
            external component <TextInput />
            component <SearchBox emits { SearchSubmitted { query:string } } /> = { <TextInput /> }
            action DoSearch = { query:string }
            let root() = { <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> /> }
            """);

        NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
            () => artifact.GenerateJSProgramModule());

        Assert.Contains(
            exception.Diagnostics,
            diagnostic => diagnostic.Message.Contains(
                "action-handler codegen is not supported",
                StringComparison.Ordinal));
    }

    [Fact]
    public void GenerateNxIr_WithProgramArtifact_ReturnsImageAndMetadata()
    {
        using NxProgramArtifact artifact = NxProgramArtifact.Build("let root() = { 1 + 2 }");

        NxGeneratedNxIr ir = artifact.GenerateNxIr();

        Assert.Equal("NXIR", Encoding.ASCII.GetString(ir.Bytes, 0, 4));
        Assert.Equal(6u, BinaryPrimitives.ReadUInt32LittleEndian(ir.Bytes.AsSpan(4, 4)));
        Assert.Equal((uint)ir.Bytes.Length, BinaryPrimitives.ReadUInt32LittleEndian(ir.Bytes.AsSpan(8, 4)));
        Assert.Equal(0, ir.Bytes.Length % 4);
        Assert.Equal(6, ir.Metadata.SchemaVersion);
        Assert.Equal("nx-ir-runtime-v2", ir.Metadata.RuntimeAbi);
        Assert.Equal("input.nx", ir.Identity);
        Assert.Equal("root", Assert.Single(ir.Metadata.FunctionEntrypoints));
        Assert.Empty(ir.Metadata.ComponentEntrypoints);

        string text = NxRuntime.ExplainNxIr(ir.Bytes);
        Assert.StartsWith($"module input.nx fingerprint {ir.Metadata.Fingerprint}", text, StringComparison.Ordinal);
        Assert.Contains("function root() =\n  (1 add 2)\n", text, StringComparison.Ordinal);

        NxGeneratedNxIr withDebug = Assert.Single(artifact.GenerateNxIr(new NxIrEmitOptions { Debug = true }));
        Assert.True(withDebug.Bytes.Length > ir.Bytes.Length);
        Assert.Contains("function root() @1:1-1:23 =", NxRuntime.ExplainNxIr(withDebug.Bytes), StringComparison.Ordinal);
    }

    [Fact]
    public void GenerateNxIr_SourceConvenienceUsesBuildContext()
    {
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        NxGeneratedNxIr ir = NxRuntime.GenerateNxIr("let root() = { 42 }", buildContext);

        Assert.Equal("NXIR", Encoding.ASCII.GetString(ir.Bytes, 0, 4));
        Assert.Equal("root", Assert.Single(ir.Metadata.FunctionEntrypoints));
    }

    [Fact]
    public void GenerateNxIr_EmitsTheConformanceCorpusImagesByteForByte()
    {
        string dir = Path.Combine(FindRepositoryRoot(), "specs", "ir-conformance", "two-module");
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();
        NxWorkspace workspace = new(
        [
            NxWorkspaceModule.FromSourceText("app/main.nx", File.ReadAllText(Path.Combine(dir, "app", "main.nx"))),
            NxWorkspaceModule.FromSourceText("shared/model.nx", File.ReadAllText(Path.Combine(dir, "shared", "model.nx"))),
        ]);
        using NxProgramArtifact artifact = NxProgramArtifact.BuildWorkspace(workspace, "app/main.nx", buildContext);

        foreach (bool debug in new[] { false, true })
        {
            IReadOnlyList<NxGeneratedNxIr> artifacts = artifact.GenerateNxIr(
                new NxIrEmitOptions { Modules = Array.Empty<string>(), Debug = debug });
            Assert.Equal(2, artifacts.Count);
            foreach (NxGeneratedNxIr entry in artifacts)
            {
                string file = entry.Identity.Replace("/", "__", StringComparison.Ordinal) + (debug ? ".nxir" : ".stripped.nxir");
                byte[] expected = File.ReadAllBytes(Path.Combine(dir, "expected", file));
                Assert.True(expected.AsSpan().SequenceEqual(entry.Bytes), $"{file} differs");
            }
        }
    }

    [Fact]
    public void ExplainNxIr_RefusesBytesThatAreNotAnImage()
    {
        NxGeneratedNxIr ir = NxRuntime.GenerateNxIr("let root() = { 42 }");

        NxEvaluationException truncated = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.ExplainNxIr(ir.Bytes.AsSpan(0, 8).ToArray()));
        Assert.Contains(truncated.Diagnostics, diagnostic => diagnostic.Code == "nx-ir-malformed");

        NxEvaluationException notAnImage = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.ExplainNxIr(Encoding.UTF8.GetBytes("{}")));
        Assert.Contains(notAnImage.Diagnostics, diagnostic => diagnostic.Message.Contains("not an NX IR image", StringComparison.Ordinal));
    }

    private static string FindRepositoryRoot()
    {
        string? directory = AppContext.BaseDirectory;
        while (directory is not null)
        {
            if (Directory.Exists(Path.Combine(directory, "specs", "ir-conformance")))
            {
                return directory;
            }

            directory = Path.GetDirectoryName(directory);
        }

        throw new InvalidOperationException("The repository root was not found above the test directory.");
    }

    [Fact]
    public void GenerateNxIr_WithIrDiagnostics_ThrowsEvaluationException()
    {
        // A conditional property fragment is a construct NX IR has no node for.
        using NxProgramArtifact artifact = NxProgramArtifact.Build(
            """
            external component <Notice density:string />
            let root(compact:boolean) = { <Notice if compact { density="tight" } else { density="normal" } /> }
            """);

        NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
            () => artifact.GenerateNxIr());

        Assert.Contains(
            exception.Diagnostics,
            diagnostic => diagnostic.Code == "codegen-unsupported-construct");
    }

    [Fact]
    public void GenerateNxIr_WithActionHandler_CarriesTheHandler()
    {
        using NxProgramArtifact artifact = NxProgramArtifact.Build(
            """
            external component <TextInput />
            component <SearchBox emits { SearchSubmitted { query:string } } /> = { <TextInput /> }
            action DoSearch = { query:string }
            let root() = { <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> /> }
            """);

        NxGeneratedNxIr ir = artifact.GenerateNxIr();

        Assert.Equal(new[] { "action-handlers-v1" }, ir.Metadata.RequiredFeatures);
        Assert.Contains(
            "onSearchSubmitted=handler SearchBox.SearchSubmitted action@0:SearchBox.SearchSubmitted =>",
            NxRuntime.ExplainNxIr(ir.Bytes),
            StringComparison.Ordinal);
    }

    [Fact]
    public void BuildProgramArtifact_WithMissingLibraryFromContext_ThrowsEvaluationException()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-prepared-invalid-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string appRoot = Path.Combine(tempPath, "app");
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(appRoot);
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                let broken(): int = "oops"
                """);

            string source = """
                import "../question-flow"
                let root() = { 0 }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);
            using NxLibraryRegistry registry = new();
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();

            NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
                () => NxProgramArtifact.Build(source, buildContext, mainPath));

            Assert.Contains(
                exception.Diagnostics,
                diagnostic => diagnostic.Message.Contains(
                    "Missing workspace module or loaded library",
                    StringComparison.Ordinal));
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void ValidateWorkspace_WithByteBackedModules_ReturnsNoDiagnostics()
    {
        NxWorkspace workspace = new([
            new NxWorkspaceModule(
                "app/main.nx",
                Encoding.UTF8.GetBytes("""
                import { answer } from "../shared/value.nx"
                let root(): int = { answer() }
                """)),
            NxWorkspaceModule.FromSourceText(
                "shared/value.nx",
                "export let answer(): int = { 42 }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        IReadOnlyList<NxDiagnostic> diagnostics = NxRuntime.ValidateWorkspace(workspace, buildContext);

        Assert.Empty(diagnostics);
    }

    [Fact]
    public void ValidateWorkspace_ReturnsStructuredDiagnostics()
    {
        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText(
                "main.nx",
                "let root(): int = { \"oops\" }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        IReadOnlyList<NxDiagnostic> diagnostics = NxRuntime.ValidateWorkspace(workspace, buildContext);

        Assert.Contains(
            diagnostics,
            diagnostic => diagnostic.Code == "return-type-mismatch");
    }

    [Fact]
    public void ValidateWorkspace_WithDuplicateNormalizedIdentity_ThrowsInteropException()
    {
        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText("shared/value.nx", "let root() = { 1 }"),
            NxWorkspaceModule.FromSourceText("shared/./value.nx", "let root() = { 2 }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        InvalidOperationException exception = Assert.Throws<InvalidOperationException>(
            () => NxRuntime.ValidateWorkspace(workspace, buildContext));

        Assert.Contains("interop arguments were invalid", exception.Message, StringComparison.Ordinal);
    }



    [Fact]
    public void GenerateNxIr_RecordsTheVersionTheWorkspaceGaveEachModule()
    {
        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText(
                "drawnui.nx",
                "export external component <SkiaLabel Text:string />",
                version: "9"),
            NxWorkspaceModule.FromSourceText("input.nx", "let root() = <SkiaLabel Text=\"hi\" />"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        using NxProgramArtifact artifact = NxProgramArtifact.BuildWorkspace(workspace, "input.nx", buildContext, ["drawnui.nx"]);
        NxGeneratedNxIr ir = Assert.Single(artifact.GenerateNxIr(new NxIrEmitOptions()));

        string text = NxRuntime.ExplainNxIr(ir.Bytes);
        Assert.Contains("links drawnui.nx version \"9\" fingerprint ", text, StringComparison.Ordinal);
    }

    [Fact]
    public void BuildWorkspace_WithMissingEntry_ThrowsEvaluationException()
    {
        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText("main.nx", "let root() = { 42 }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
            () => NxProgramArtifact.BuildWorkspace(workspace, "missing.nx", buildContext));

        Assert.Contains(
            exception.Diagnostics,
            diagnostic => diagnostic.Code == "workspace-entry-not-found");
    }





    [Fact]
    public void WorkspaceApis_ValidateArgumentsBeforeNativeCall()
    {
        Assert.Throws<ArgumentException>(
            () => NxWorkspaceModule.FromSourceText(string.Empty, "let root() = { 42 }"));

        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText("main.nx", "let root() = { 42 }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();

        Assert.Throws<ArgumentNullException>(
            () => NxRuntime.ValidateWorkspace(null!, buildContext));
        Assert.Throws<ArgumentNullException>(
            () => NxRuntime.ValidateWorkspace(workspace, null!));
        Assert.Throws<ArgumentException>(
            () => NxProgramArtifact.BuildWorkspace(workspace, string.Empty, buildContext));
    }



    [Fact]
    public void LibraryRegistry_LoadFromDirectory_WithInvalidSource_ThrowsEvaluationException()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-library-artifact-invalid-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);

        try
        {
            string libraryRoot = Path.Combine(tempPath, "question-flow");
            Directory.CreateDirectory(libraryRoot);
            File.WriteAllText(
                Path.Combine(libraryRoot, "QuestionFlow.nx"),
                """
                let broken(): int = "oops"
                """);

            using NxLibraryRegistry registry = new();
            NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
                () => registry.LoadFromDirectory(libraryRoot));

            Assert.Contains(
                exception.Diagnostics,
                diagnostic => diagnostic.Code == "return-type-mismatch");
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }
}
