// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Linq;
using NxLang.Nx;
using Xunit;

namespace NxLang.Nx.Tests;

public class NxRuntimeErrorTests
{
    [Fact]
    public void Evaluate_SyntaxError_ThrowsNxEvaluationException()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source));

        Assert.NotEmpty(ex.Diagnostics);
        Assert.All(ex.Diagnostics, d => Assert.Equal(NxSeverity.Error, d.Severity));
    }

    [Fact]
    public void Evaluate_MissingRootFunction_ThrowsNxEvaluationException()
    {
        string source = "let foo() = { 42 }";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source));

        Assert.NotEmpty(ex.Diagnostics);
    }

    [Fact]
    public void Evaluate_SyntaxError_DiagnosticsHaveCorrectStructure()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source));

        Assert.NotEmpty(ex.Diagnostics);
        NxDiagnostic diagnostic = ex.Diagnostics[0];

        Assert.Equal(NxSeverity.Error, diagnostic.Severity);
        Assert.NotEmpty(diagnostic.Message);
        Assert.NotNull(diagnostic.Labels);
    }

    [Fact]
    public void Evaluate_SyntaxError_DiagnosticLabelsHaveCorrectStructure()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source));

        Assert.NotEmpty(ex.Diagnostics);
        NxDiagnostic diagnostic = ex.Diagnostics[0];

        if (diagnostic.Labels.Length > 0)
        {
            NxDiagnosticLabel label = diagnostic.Labels[0];

            Assert.NotNull(label.File);
            Assert.NotNull(label.Span);
            Assert.True(label.Span.StartLine >= 1);
            Assert.True(label.Span.StartColumn >= 1);
        }
    }

    [Fact]
    public void Evaluate_WithFileName_DiagnosticLabelsContainFileName()
    {
        string source = "let x = ";
        string fileName = "custom.nx";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source, fileName));

        Assert.NotEmpty(ex.Diagnostics);
        NxDiagnostic diagnostic = ex.Diagnostics[0];

        if (diagnostic.Labels.Length > 0)
        {
            NxDiagnosticLabel label = diagnostic.Labels[0];
            Assert.Equal(fileName, label.File);
        }
    }

    [Fact]
    public void Evaluate_StaticAnalysisDiagnostics_AreAggregated_AndKeepFileName()
    {
        string source = """
            abstract type Entity = {
              id: int
            }

            type User extends Entity = {
              name: string
            }

            type Admin extends User = {
              level: int
            }

            let broken(): int = "oops"
            let root(): int = { 1 / 0 }
            """;
        string fileName = "widgets/search-box.nx";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.Evaluate<int>(source, fileName));

        Assert.Contains(ex.Diagnostics, d => d.Code == "lowering-error");
        Assert.Contains(ex.Diagnostics, d => d.Code == "return-type-mismatch");
        Assert.DoesNotContain(ex.Diagnostics, d => d.Code == "runtime-error");

        NxDiagnostic[] staticDiagnostics = ex.Diagnostics
            .Where(d => d.Code == "lowering-error" || d.Code == "return-type-mismatch")
            .ToArray();
        Assert.NotEmpty(staticDiagnostics);
        Assert.All(
            staticDiagnostics,
            diagnostic => Assert.Equal(fileName, diagnostic.Labels[0].File));
    }

    [Fact]
    public void EvaluateBytes_SyntaxError_ThrowsNxEvaluationException()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.EvaluateBytes(source));

        Assert.NotEmpty(ex.Diagnostics);
    }

    [Fact]
    public void EvaluateBytes_JsonOutput_SyntaxError_ThrowsNxEvaluationException()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.EvaluateBytes(source, NxOutputFormat.Json));

        Assert.NotEmpty(ex.Diagnostics);
        Assert.All(ex.Diagnostics, diagnostic => Assert.Equal(NxSeverity.Error, diagnostic.Severity));
    }

    [Fact]
    public void NxRuntime_DoesNotExposeLegacyJsonConverterHelpers()
    {
        Assert.Null(typeof(NxRuntime).GetMethod("ValueBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("DiagnosticsBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("ComponentInitResultBytesToJson"));
        Assert.Null(typeof(NxRuntime).GetMethod("ComponentDispatchResultBytesToJson"));
    }
}
