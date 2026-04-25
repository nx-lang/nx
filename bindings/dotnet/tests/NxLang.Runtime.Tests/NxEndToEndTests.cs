// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using NxLang.Nx;
using NxLang.Nx.Interop;
using Xunit;

namespace NxLang.Nx.Tests;

public class NxEndToEndTests
{
    [Fact]
    public void Evaluate_ComplexExpression_ReturnsCorrectValue()
    {
        string source = "let root() = { 10 + 32 }";

        int result = NxRuntime.Evaluate<int>(source);

        Assert.Equal(42, result);
    }

    [Fact]
    public void Evaluate_WithCustomFileName_UsesFileNameInDiagnostics()
    {
        string source = "let x = ";
        string customFileName = "my-custom-file.nx";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source, customFileName));

        Assert.NotEmpty(ex.Diagnostics);
        if (ex.Diagnostics[0].Labels.Length > 0)
        {
            Assert.Equal(customFileName, ex.Diagnostics[0].Labels[0].File);
        }
    }

    [Fact]
    public void Evaluate_ConcurrentEvaluations_AllSucceed()
    {
        string source = "let root() = { 42 }";

        Parallel.For(0, 10, _ =>
        {
            int result = NxRuntime.Evaluate<int>(source);
            Assert.Equal(42, result);
        });
    }

    [Fact]
    public void Evaluate_DifferentTypes_AllSucceed()
    {
        Assert.Equal(42, NxRuntime.Evaluate<int>("let root() = { 42 }"));
        Assert.Equal("text", NxRuntime.Evaluate<string>("let root() = { \"text\" }"));
        Assert.True(NxRuntime.Evaluate<bool>("let root() = { true }"));
    }

    [Fact]
    public void EvaluateBytes_JsonOutput_ComplexExpression_ReturnsValidJson()
    {
        string source = "let root() = { 10 + 32 }";

        byte[] resultBytes = NxRuntime.EvaluateBytes(source, NxOutputFormat.Json);
        string json = Encoding.UTF8.GetString(resultBytes);

        Assert.Equal("42", json);
    }

    [Fact]
    public void NativeLibrary_IsStagedAlongsideTestOutput()
    {
        string nativeLibraryPath = Path.Combine(AppContext.BaseDirectory, NxNativeLibraryInfo.GetFileName());

        Assert.True(
            File.Exists(nativeLibraryPath),
            $"Expected the staged NX native runtime at '{nativeLibraryPath}'. Build `cargo build --release -p nx-ffi` before running dotnet tests.");
    }

    [Fact]
    public void Evaluate_WithProgramArtifact_ReusesPreloadedLibraryAcrossBuildContexts()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-prepared-{Guid.NewGuid():N}");
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
                export let answer() = { 42 }
                """);

            string source = """
                import "../question-flow"
                let root() = { answer() }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);

            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext firstContext = registry.CreateBuildContext();
            using NxProgramBuildContext secondContext = registry.CreateBuildContext();
            using NxProgramArtifact firstProgram = NxProgramArtifact.Build(source, firstContext, mainPath);
            using NxProgramArtifact secondProgram = NxProgramArtifact.Build(source, secondContext, mainPath);

            int firstResult = NxRuntime.Evaluate<int>(firstProgram);
            int secondResult = NxRuntime.Evaluate<int>(secondProgram);

            Assert.Equal(42, firstResult);
            Assert.Equal(42, secondResult);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void Evaluate_WithProgramArtifact_RemainsExecutableAfterBuildContextAndRegistryDispose()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-prepared-disposed-{Guid.NewGuid():N}");
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
                export let answer() = { 42 }
                """);

            string source = """
                import "../question-flow"
                let root() = { answer() }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);

            NxProgramArtifact programArtifact;
            using (NxLibraryRegistry registry = new())
            {
                registry.LoadFromDirectory(libraryRoot);
                using NxProgramBuildContext buildContext = registry.CreateBuildContext();
                programArtifact = NxProgramArtifact.Build(source, buildContext, mainPath);
            }

            using (programArtifact)
            {
                int result = NxRuntime.Evaluate<int>(programArtifact);
                Assert.Equal(42, result);
            }
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
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
    public void Evaluate_WithWorkspaceProgramArtifact_UsesSelectedEntryRoot()
    {
        NxWorkspace workspace = new([
            NxWorkspaceModule.FromSourceText("a.nx", "let root() = { \"a\" }"),
            NxWorkspaceModule.FromSourceText("b.nx", "let root() = { \"b\" }"),
        ]);
        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();
        using NxProgramArtifact artifact = NxProgramArtifact.BuildWorkspace(workspace, "b.nx", buildContext);

        string result = NxRuntime.Evaluate<string>(artifact);

        Assert.Equal("b", result);
    }

    [Fact]
    public void Evaluate_WithWorkspaceProgramArtifact_RemainsExecutableAfterWorkspaceBuffersAreReleased()
    {
        NxProgramArtifact artifact;
        using (NxLibraryRegistry registry = new())
        {
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();
            byte[] source = Encoding.UTF8.GetBytes("let root() = { 42 }");
            NxWorkspace workspace = new([
                new NxWorkspaceModule("main.nx", source),
            ]);
            artifact = NxProgramArtifact.BuildWorkspace(workspace, "main.nx", buildContext);
        }

        using (artifact)
        {
            int result = NxRuntime.Evaluate<int>(artifact);
            Assert.Equal(42, result);
        }
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
    public void LibraryRegistry_LoadFromDirectory_Succeeds()
    {
        string tempPath = Path.Combine(Path.GetTempPath(), $"nx-library-artifact-{Guid.NewGuid():N}");
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
                export let answer() = { 42 }
                """);
            string source = """
                import "../question-flow"
                let root() = { answer() }
                """;
            string mainPath = Path.Combine(appRoot, "main.nx");
            File.WriteAllText(mainPath, source);

            using NxLibraryRegistry registry = new();
            registry.LoadFromDirectory(libraryRoot);
            using NxProgramBuildContext buildContext = registry.CreateBuildContext();

            int result = NxRuntime.Evaluate<int>(source, buildContext, mainPath);

            Assert.Equal(42, result);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
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
