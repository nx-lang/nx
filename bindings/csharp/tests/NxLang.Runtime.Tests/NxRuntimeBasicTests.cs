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
    public void EvaluateToMessagePack_SimpleInteger_ReturnsCorrectBytes()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateToMessagePack(source);

        int value = MessagePackSerializer.Deserialize<int>(result);
        Assert.Equal(42, value);
    }

    [Fact]
    public void EvaluateToJson_SimpleInteger_ReturnsCorrectJson()
    {
        string source = "let root() = { 42 }";

        string result = NxRuntime.EvaluateToJson(source);

        Assert.Equal("42", result);
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
    public void EvaluateToMessagePack_NullSource_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => NxRuntime.EvaluateToMessagePack(null!));
    }

    [Fact]
    public void EvaluateToJson_NullSource_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => NxRuntime.EvaluateToJson(null!));
    }

    [Fact]
    public void Evaluate_NullSource_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => NxRuntime.Evaluate<int>(null!));
    }

    [Fact]
    public void EvaluateToMessagePack_WithFileName_DoesNotThrow()
    {
        string source = "let root() = { 42 }";

        byte[] result = NxRuntime.EvaluateToMessagePack(source, "test.nx");

        int value = MessagePackSerializer.Deserialize<int>(result);
        Assert.Equal(42, value);
    }

    [Fact]
    public void EvaluateToJson_WithFileName_DoesNotThrow()
    {
        string source = "let root() = { 42 }";

        string result = NxRuntime.EvaluateToJson(source, "test.nx");

        Assert.Equal("42", result);
    }
}
