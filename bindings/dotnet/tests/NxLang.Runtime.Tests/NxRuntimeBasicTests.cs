// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text;
using System.Text.Json;
using MessagePack;
using NxLang.Nx;
using Xunit;

namespace NxLang.Nx.Tests;

public class NxRuntimeBasicTests
{
    [Fact]
    public void EvaluateBytes_SimpleInteger_ReturnsCorrectBytes()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateBytes(source);

        int value = MessagePackSerializer.Deserialize<int>(result, cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(42, value);
    }

    [Fact]
    public void EvaluateBytes_JsonOutput_ReturnsCorrectJson()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateBytes(source, NxOutputFormat.Json);
        string json = Encoding.UTF8.GetString(result);

        Assert.Equal("42", json);
    }

    [Fact]
    public void EvaluateJson_SimpleInteger_ReturnsCorrectJsonElement()
    {
        string source = "let root() = { 42 }";

        JsonElement result = NxRuntime.EvaluateJson(source);

        Assert.Equal(42, result.GetInt32());
    }

    [Fact]
    public void EvaluateJson_WithBuildContext_ReturnsCorrectJsonElement()
    {
        string source = "let root() = { 42 }";

        using NxLibraryRegistry registry = new();
        using NxProgramBuildContext buildContext = registry.CreateBuildContext();
        JsonElement result = NxRuntime.EvaluateJson(source, buildContext);

        Assert.Equal(42, result.GetInt32());
    }

    [Fact]
    public void EvaluateJson_WithProgramArtifact_ReturnsCorrectJsonElement()
    {
        string source = "let root() = { 42 }";

        using NxProgramArtifact programArtifact = NxProgramArtifact.Build(source);
        JsonElement result = NxRuntime.EvaluateJson(programArtifact);

        Assert.Equal(42, result.GetInt32());
    }

    [Fact]
    public void EvaluateJson_EnumValue_ReturnsBareAuthoredMemberString()
    {
        string source = """
            enum ThemeMode = | light | dark

            let root() = { ThemeMode.dark }
            """;

        JsonElement result = NxRuntime.EvaluateJson(source);

        Assert.Equal(JsonValueKind.String, result.ValueKind);
        Assert.Equal("dark", result.GetString());
    }

    [Fact]
    public void Evaluate_SimpleInteger_ReturnsCorrectValue()
    {
        string source = "let root() = { 42 }";

        int result = NxRuntime.Evaluate<int>(source);

        Assert.Equal(42, result);
    }

    [Fact]
    public void Evaluate_SimpleString_ReturnsCorrectValue()
    {
        string source = "let root() = { \"Hello, NX!\" }";

        string result = NxRuntime.Evaluate<string>(source);

        Assert.Equal("Hello, NX!", result);
    }

    [Fact]
    public void EvaluateBytes_NullSource_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => NxRuntime.EvaluateBytes(null!));
    }

    [Fact]
    public void Evaluate_NullSource_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => NxRuntime.Evaluate<int>(null!));
    }

    [Fact]
    public void EvaluateBytes_WithFileName_DoesNotThrow()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateBytes(source, "test.nx");

        int value = MessagePackSerializer.Deserialize<int>(result, cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(42, value);
    }

    [Fact]
    public void EvaluateBytes_JsonOutput_WithFileName_DoesNotThrow()
    {
        string source = "let root() = { 42 }";

        byte[] resultBytes = NxRuntime.EvaluateBytes(source, NxOutputFormat.Json, "test.nx");
        string result = Encoding.UTF8.GetString(resultBytes);

        Assert.Equal("42", result);
    }
}
