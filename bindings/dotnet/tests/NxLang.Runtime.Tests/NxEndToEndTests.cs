// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.IO;
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
    public void ValueBytesToJson_ComplexExpression_ReturnsValidJson()
    {
        string source = "let root() = { 10 + 32 }";

        byte[] resultBytes = NxRuntime.EvaluateBytes(source);
        string json = NxRuntime.ValueBytesToJson(resultBytes);

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
                let answer() = { 42 }
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
                let answer() = { 42 }
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
                diagnostic => diagnostic.Message.Contains("Missing loaded library", StringComparison.Ordinal));
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
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
                let answer() = { 42 }
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
