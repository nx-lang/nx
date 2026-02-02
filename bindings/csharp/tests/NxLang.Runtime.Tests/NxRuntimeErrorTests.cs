// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
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
        Assert.All(ex.Diagnostics, d => Assert.Equal("error", d.Severity));
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

        Assert.NotNull(diagnostic.Severity);
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
    public void EvaluateToJson_SyntaxError_ThrowsNxEvaluationException()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.EvaluateToJson(source));

        Assert.NotEmpty(ex.Diagnostics);
    }

    [Fact]
    public void EvaluateToMessagePack_SyntaxError_ThrowsNxEvaluationException()
    {
        string source = "let x = ";

        NxEvaluationException ex = Assert.Throws<NxEvaluationException>(
            () => NxRuntime.EvaluateToMessagePack(source));

        Assert.NotEmpty(ex.Diagnostics);
    }
}
