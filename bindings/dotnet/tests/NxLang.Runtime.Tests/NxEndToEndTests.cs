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
    public void Evaluate_WithProgramArtifact_ReusesImportedLibraryContext()
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
            using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source, mainPath);

            int result = NxRuntime.Evaluate<int>(programArtifact);

            Assert.Equal(42, result);
        }
        finally
        {
            Directory.Delete(tempPath, recursive: true);
        }
    }

    [Fact]
    public void BuildProgramArtifact_WithInvalidImportedLibrary_ThrowsEvaluationException()
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

            NxEvaluationException exception = Assert.Throws<NxEvaluationException>(
                () => NxProgramArtifact.Build(source, mainPath));

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
