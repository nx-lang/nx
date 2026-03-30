// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
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
    public void ValueBytesToJson_SimpleInteger_ReturnsCorrectJson()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateBytes(source);
        string json = NxRuntime.ValueBytesToJson(result);

        Assert.Equal("42", json);
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
    public void ValueBytesToJson_WithFileName_DoesNotThrow()
    {
        string source = "let root() = { 42 }";

        byte[] resultBytes = NxRuntime.EvaluateBytes(source, "test.nx");
        string result = NxRuntime.ValueBytesToJson(resultBytes);

        Assert.Equal("42", result);
    }
}
