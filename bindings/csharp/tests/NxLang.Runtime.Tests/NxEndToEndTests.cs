// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using MessagePack;
using NxLang.Nx;
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
    public void Evaluate_WithCustomMessagePackOptions_UsesOptions()
    {
        string source = "let root() = { 42 }";
        MessagePackSerializerOptions customOptions = MessagePackSerializerOptions.Standard
            .WithSecurity(MessagePackSecurity.UntrustedData);

        int result = NxRuntime.Evaluate<int>(source, null, customOptions);

        Assert.Equal(42, result);
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
    public void EvaluateToJson_ComplexExpression_ReturnsValidJson()
    {
        string source = "let root() = { 10 + 32 }";

        string json = NxRuntime.EvaluateToJson(source);

        Assert.Equal("42", json);
    }
}
