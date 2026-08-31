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

// A mixed union: `idle` declares no fields and the union declares no base, so it is a constant
// case and its wire form is the bare string "idle". `failed` carries a payload and keeps the
// `$type` map. The converter is what admits both shapes.
[JsonConverter(typeof(NxPolymorphicJsonConverter<LoadState>))]
[NxUnionCase(typeof(LoadStateIdle), "LoadState.idle")]
[NxUnionCase(typeof(LoadStateFailed), "LoadState.failed")]
[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<LoadState>))]
internal abstract class LoadState
{
}

[NxConstantCase("idle")]
[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<LoadState, LoadStateIdle>))]
internal sealed class LoadStateIdle : LoadState
{
    public static readonly LoadStateIdle Instance = new();
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

    /// <summary>
    /// A constant case carries the bare authored string on the wire, and it does so whether its
    /// union is wholly constant or mixes constant cases with payload ones. Those two declarations
    /// are the same value shape, which is why one declaration form suffices for both.
    /// </summary>
    [Fact]
    public void RawConstantCaseResults_UseTheBareStringWireShape()
    {
        JsonElement constantUnionJson = NxRuntime.EvaluateJson(ConstantUnionSource);
        JsonElement mixedUnionJson = NxRuntime.EvaluateJson(FieldlessUnionSource);
        byte[] constantUnionBytes = NxRuntime.EvaluateBytes(ConstantUnionSource);
        byte[] mixedUnionBytes = NxRuntime.EvaluateBytes(FieldlessUnionSource);

        Assert.Equal(JsonValueKind.String, constantUnionJson.ValueKind);
        Assert.Equal("closed", constantUnionJson.GetString());
        Assert.Equal(JsonValueKind.String, mixedUnionJson.ValueKind);
        Assert.Equal("idle", mixedUnionJson.GetString());

        Assert.Equal(
            "closed",
            MessagePackSerializer.Deserialize<string>(
                constantUnionBytes,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(
            "idle",
            MessagePackSerializer.Deserialize<string>(
                mixedUnionBytes,
                cancellationToken: TestContext.Current.CancellationToken));
    }

    /// <summary>A mixed union round-trips through JSON in both of its shapes.</summary>
    [Fact]
    public void TypedJsonMixedUnion_RoundTripsBothShapes()
    {
        string constantJson = JsonSerializer.Serialize<LoadState>(LoadStateIdle.Instance);
        Assert.Equal("\"idle\"", constantJson);

        string payloadJson = JsonSerializer.Serialize<LoadState>(new LoadStateFailed { Message = "Offline" });
        using (JsonDocument document = JsonDocument.Parse(payloadJson))
        {
            Assert.Equal("LoadState.failed", document.RootElement.GetProperty("$type").GetString());
        }

        Assert.IsType<LoadStateIdle>(JsonSerializer.Deserialize<LoadState>(constantJson));
        LoadStateFailed failed = Assert.IsType<LoadStateFailed>(
            JsonSerializer.Deserialize<LoadState>(payloadJson));
        Assert.Equal("Offline", failed.Message);
    }

    /// <summary>A mixed union round-trips through MessagePack in both of its shapes.</summary>
    [Fact]
    public void TypedMessagePackMixedUnion_RoundTripsBothShapes()
    {
        byte[] constantBytes = MessagePackSerializer.Serialize<LoadState>(
            LoadStateIdle.Instance,
            cancellationToken: TestContext.Current.CancellationToken);
        MessagePackReader constantReader = new(new ReadOnlySequence<byte>(constantBytes));
        Assert.Equal(MessagePackType.String, constantReader.NextMessagePackType);
        Assert.Equal(
            "idle",
            MessagePackSerializer.Deserialize<string>(
                constantBytes,
                cancellationToken: TestContext.Current.CancellationToken));

        byte[] payloadBytes = MessagePackSerializer.Serialize<LoadState>(
            new LoadStateFailed { Message = "Offline" },
            cancellationToken: TestContext.Current.CancellationToken);
        MessagePackReader payloadReader = new(new ReadOnlySequence<byte>(payloadBytes));
        Assert.Equal(MessagePackType.Map, payloadReader.NextMessagePackType);

        Assert.IsType<LoadStateIdle>(
            MessagePackSerializer.Deserialize<LoadState>(
                constantBytes,
                cancellationToken: TestContext.Current.CancellationToken));
        LoadStateFailed failed = Assert.IsType<LoadStateFailed>(
            MessagePackSerializer.Deserialize<LoadState>(
                payloadBytes,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal("Offline", failed.Message);
    }

    /// <summary>An unknown bare case name is rejected rather than silently accepted.</summary>
    [Fact]
    public void UnknownConstantCaseName_IsRejected()
    {
        Assert.Throws<JsonException>(
            () => JsonSerializer.Deserialize<LoadState>("\"sparkly\""));

        byte[] unknown = MessagePackSerializer.Serialize(
            "sparkly",
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<LoadState>(
                unknown,
                cancellationToken: TestContext.Current.CancellationToken));
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

    private const string ConstantUnionSource = """
        type CardSortMode = open | closed

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
