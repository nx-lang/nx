// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using NxLang.Nx.Serialization;
using Xunit;

namespace NxLang.Nx.Tests;

[JsonConverter(typeof(NxEnumJsonConverter<CardSortMode, CardSortModeWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<CardSortMode, CardSortModeWireFormat>))]
internal enum CardSortMode
{
    Open,
    Closed,
}

internal sealed class CardSortModeWireFormat : INxEnumWireFormat<CardSortMode>
{
    public static string Format(CardSortMode value)
    {
        return value switch
        {
            CardSortMode.Open => "open",
            CardSortMode.Closed => "closed",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static CardSortMode Parse(string value)
    {
        return value switch
        {
            "open" => CardSortMode.Open,
            "closed" => CardSortMode.Closed,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(LoadStateIdle), "LoadState.idle")]
[JsonDerivedType(typeof(LoadStateFailed), "LoadState.failed")]
[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<LoadState>))]
internal abstract class LoadState
{
}

[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<LoadState, LoadStateIdle>))]
internal sealed class LoadStateIdle : LoadState
{
}

[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<LoadState, LoadStateFailed>))]
internal sealed class LoadStateFailed : LoadState
{
    [Key("message")]
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}

public class NxUnionSerializationTests
{
    [Fact]
    public void EvaluateJson_PayloadUnionCase_ReturnsTypeMap()
    {
        JsonElement result = NxRuntime.EvaluateJson(PayloadUnionSource);

        Assert.Equal(JsonValueKind.Object, result.ValueKind);
        Assert.Equal("LoadState.failed", result.GetProperty("$type").GetString());
        Assert.Equal("Offline", result.GetProperty("message").GetString());
    }

    [Fact]
    public void EvaluateBytes_PayloadUnionCase_ReturnsMessagePackTypeMap()
    {
        byte[] result = NxRuntime.EvaluateBytes(PayloadUnionSource);
        MessagePackReader reader = new(new ReadOnlySequence<byte>(result));

        Assert.Equal(MessagePackType.Map, reader.NextMessagePackType);

        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                result,
                cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("LoadState.failed", Assert.IsType<string>(payload["$type"]));
        Assert.Equal("Offline", Assert.IsType<string>(payload["message"]));
    }

    [Fact]
    public void TypedJsonUnionCase_SerializesAsTypeMap()
    {
        LoadState state = new LoadStateFailed
        {
            Message = "Offline",
        };

        string json = JsonSerializer.Serialize(state);
        using JsonDocument document = JsonDocument.Parse(json);
        JsonElement payload = document.RootElement;

        Assert.Equal(JsonValueKind.Object, payload.ValueKind);
        Assert.Equal("LoadState.failed", payload.GetProperty("$type").GetString());
        Assert.Equal("Offline", payload.GetProperty("message").GetString());
    }

    [Fact]
    public void TypedMessagePackUnionCase_SerializesAsTypeMap()
    {
        LoadState state = new LoadStateFailed
        {
            Message = "Offline",
        };

        byte[] result = MessagePackSerializer.Serialize(
            state,
            cancellationToken: TestContext.Current.CancellationToken);
        MessagePackReader reader = new(new ReadOnlySequence<byte>(result));

        Assert.Equal(MessagePackType.Map, reader.NextMessagePackType);

        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                result,
                cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("LoadState.failed", Assert.IsType<string>(payload["$type"]));
        Assert.Equal("Offline", Assert.IsType<string>(payload["message"]));
    }

    [Fact]
    public void TypedMessagePackConcreteUnionCase_SerializesAsTypeMap()
    {
        LoadStateFailed state = new()
        {
            Message = "Offline",
        };

        byte[] result = MessagePackSerializer.Serialize(
            state,
            cancellationToken: TestContext.Current.CancellationToken);
        MessagePackReader reader = new(new ReadOnlySequence<byte>(result));

        Assert.Equal(MessagePackType.Map, reader.NextMessagePackType);

        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                result,
                cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("LoadState.failed", Assert.IsType<string>(payload["$type"]));
        Assert.Equal("Offline", Assert.IsType<string>(payload["message"]));
    }

    [Fact]
    public void TypedJsonUnionCase_DeserializesFromTypeMap()
    {
        string json = """
            {
              "$type": "LoadState.failed",
              "message": "Offline"
            }
            """;

        LoadState? state = JsonSerializer.Deserialize<LoadState>(json);

        LoadStateFailed failed = Assert.IsType<LoadStateFailed>(state);
        Assert.Equal("Offline", failed.Message);
    }

    [Fact]
    public void TypedMessagePackUnionCase_DeserializesFromTypeMap()
    {
        byte[] result = BuildUnionMapBytes(
            ("$type", "LoadState.failed"),
            ("message", "Offline"));

        LoadState state = MessagePackSerializer.Deserialize<LoadState>(
            result,
            cancellationToken: TestContext.Current.CancellationToken);

        LoadStateFailed failed = Assert.IsType<LoadStateFailed>(state);
        Assert.Equal("Offline", failed.Message);
    }

    [Fact]
    public void EvaluateTypedUnionCase_DeserializesRuntimeProducedTypeMap()
    {
        LoadState state = NxRuntime.Evaluate<LoadState>(PayloadUnionSource);

        LoadStateFailed failed = Assert.IsType<LoadStateFailed>(state);
        Assert.Equal("Offline", failed.Message);
    }

    [Fact]
    public void RawEnumAndFieldlessUnionResults_UseDifferentWireShapes()
    {
        JsonElement enumJson = NxRuntime.EvaluateJson(EnumSource);
        JsonElement unionJson = NxRuntime.EvaluateJson(FieldlessUnionSource);
        byte[] enumBytes = NxRuntime.EvaluateBytes(EnumSource);
        byte[] unionBytes = NxRuntime.EvaluateBytes(FieldlessUnionSource);
        MessagePackReader unionReader = new(new ReadOnlySequence<byte>(unionBytes));

        Assert.Equal(JsonValueKind.String, enumJson.ValueKind);
        Assert.Equal("closed", enumJson.GetString());
        Assert.Equal(JsonValueKind.Object, unionJson.ValueKind);
        Assert.Equal("LoadState.idle", unionJson.GetProperty("$type").GetString());
        Assert.Equal(
            "closed",
            MessagePackSerializer.Deserialize<string>(
                enumBytes,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(MessagePackType.Map, unionReader.NextMessagePackType);

        Dictionary<string, object?> payload =
            MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
                unionBytes,
                cancellationToken: TestContext.Current.CancellationToken);

        Assert.Single(payload);
        Assert.Equal("LoadState.idle", Assert.IsType<string>(payload["$type"]));
    }

    [Fact]
    public void TypedEnumWorkflow_RemainsBareStringBased()
    {
        string json = JsonSerializer.Serialize(CardSortMode.Closed);
        byte[] bytes = MessagePackSerializer.Serialize(
            CardSortMode.Closed,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("\"closed\"", json);
        Assert.Equal(CardSortMode.Closed, JsonSerializer.Deserialize<CardSortMode>(json));
        Assert.Equal(
            "closed",
            MessagePackSerializer.Deserialize<string>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(
            CardSortMode.Closed,
            MessagePackSerializer.Deserialize<CardSortMode>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));
    }

    private const string PayloadUnionSource = """
        type LoadState =
          | idle
          | failed { message:string }

        let root(): LoadState = { <LoadState.failed message={"Offline"} /> }
        """;

    private const string FieldlessUnionSource = """
        type LoadState =
          | idle
          | failed { message:string }

        let root(): LoadState = { LoadState.idle }
        """;

    private const string EnumSource = """
        enum CardSortMode = | open | closed

        let root() = { CardSortMode.closed }
        """;

    private static byte[] BuildUnionMapBytes(params (string Key, string Value)[] entries)
    {
        ArrayBufferWriter<byte> buffer = new();
        MessagePackWriter writer = new(buffer);
        writer.WriteMapHeader(entries.Length);
        foreach ((string key, string value) in entries)
        {
            writer.Write(key);
            writer.Write(value);
        }

        writer.Flush();
        return buffer.WrittenSpan.ToArray();
    }
}
