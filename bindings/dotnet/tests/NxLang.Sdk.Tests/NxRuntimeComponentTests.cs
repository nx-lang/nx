// Copyright (c) The NX Authors.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Buffers;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using NxLang.Nx.Serialization;
using Xunit;

namespace NxLang.Nx.Tests;

[JsonConverter(typeof(NxEnumJsonConverter<ComponentDealStage, ComponentDealStageWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<ComponentDealStage, ComponentDealStageWireFormat>))]
public enum ComponentDealStage
{
    Draft,
    PendingReview,
    ClosedWon,
}

internal sealed class ComponentDealStageWireFormat : INxEnumWireFormat<ComponentDealStage>
{
    public static string Format(ComponentDealStage value)
    {
        return value switch
        {
            ComponentDealStage.Draft => "draft",
            ComponentDealStage.PendingReview => "pending_review",
            ComponentDealStage.ClosedWon => "closed_won",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static ComponentDealStage Parse(string value)
    {
        return value switch
        {
            "draft" => ComponentDealStage.Draft,
            "pending_review" => ComponentDealStage.PendingReview,
            "closed_won" => ComponentDealStage.ClosedWon,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

[JsonConverter(typeof(NxEnumJsonConverter<RestrictedDealStage, RestrictedDealStageWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<RestrictedDealStage, RestrictedDealStageWireFormat>))]
public enum RestrictedDealStage
{
    Draft,
    ClosedWon,
}

internal sealed class RestrictedDealStageWireFormat : INxEnumWireFormat<RestrictedDealStage>
{
    public static string Format(RestrictedDealStage value)
    {
        return value switch
        {
            RestrictedDealStage.Draft => "draft",
            RestrictedDealStage.ClosedWon => "closed_won",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static RestrictedDealStage Parse(string value)
    {
        return value switch
        {
            "draft" => RestrictedDealStage.Draft,
            "closed_won" => RestrictedDealStage.ClosedWon,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

[MessagePackObject]
public sealed class SearchBoxProps
{
    [Key("placeholder")]
    [JsonPropertyName("placeholder")]
    public string Placeholder { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class SearchBoxState
{
    [Key("query")]
    [JsonPropertyName("query")]
    public string Query { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class TextInputElement
{
    [Key("value")]
    [JsonPropertyName("value")]
    public string Value { get; set; } = string.Empty;

    [Key("placeholder")]
    [JsonPropertyName("placeholder")]
    public string Placeholder { get; set; } = string.Empty;
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(SearchSubmitted), "SearchSubmitted")]
[JsonDerivedType(typeof(ValueChanged), "ValueChanged")]
[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<SearchAction>))]
public abstract class SearchAction
{
}

[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<SearchAction, SearchSubmitted>))]
public sealed class SearchSubmitted : SearchAction
{
    [Key("searchString")]
    [JsonPropertyName("searchString")]
    public string SearchString { get; set; } = string.Empty;
}

[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<SearchAction, ValueChanged>))]
public sealed class ValueChanged : SearchAction
{
    [Key("value")]
    [JsonPropertyName("value")]
    public string Value { get; set; } = string.Empty;
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(ShortTextQuestion), "ShortTextQuestion")]
[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<Question>))]
public abstract class Question
{
}

[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<Question, ShortTextQuestion>))]
public sealed class ShortTextQuestion : Question
{
    [Key("label")]
    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class SurveyPayload
{
    [Key("primaryAction")]
    [JsonPropertyName("primaryAction")]
    public SearchAction PrimaryAction { get; set; } = new SearchSubmitted();

    [Key("actions")]
    [JsonPropertyName("actions")]
    public SearchAction[] Actions { get; set; } = Array.Empty<SearchAction>();

    [Key("question")]
    [JsonPropertyName("question")]
    public Question Question { get; set; } = new ShortTextQuestion();
}

[MessagePackObject]
public sealed class ThemeModeProps
{
    [Key("theme")]
    [JsonPropertyName("theme")]
    public string Theme { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class ThemeModeElement
{
    [Key("theme")]
    [JsonPropertyName("theme")]
    public string Theme { get; set; } = string.Empty;
}

[MessagePackObject]
public sealed class DealStageProps
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public ComponentDealStage Stage { get; set; } = ComponentDealStage.Draft;
}

[MessagePackObject]
public sealed class DealStageState
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public ComponentDealStage Stage { get; set; } = ComponentDealStage.Draft;
}

[MessagePackObject]
public sealed class DealStageElement
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public ComponentDealStage Stage { get; set; } = ComponentDealStage.Draft;
}

[MessagePackObject]
public sealed class RestrictedDealStageElement
{
    [Key("stage")]
    [JsonPropertyName("stage")]
    public RestrictedDealStage Stage { get; set; } = RestrictedDealStage.Draft;
}

[MessagePackObject]
public sealed class FlowState
{
    [Key("label")]
    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;
}

public class NxRuntimeComponentTests
{


















    [Fact]
    public void PolymorphicTypedAction_SerializesAsMapWithTypeDiscriminator()
    {
        SearchAction action = new SearchSubmitted
        {
            SearchString = "docs"
        };

        byte[] bytes = MessagePackSerializer.Serialize(
            action,
            cancellationToken: TestContext.Current.CancellationToken);
        MessagePackReader reader = new(new ReadOnlySequence<byte>(bytes));
        Assert.Equal(MessagePackType.Map, reader.NextMessagePackType);

        var payload = MessagePackSerializer.Deserialize<Dictionary<string, object?>>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal("SearchSubmitted", Assert.IsType<string>(payload["$type"]));
        Assert.True(payload.ContainsKey("searchString"));
    }

    [Fact]
    public void PolymorphicTypedAction_RawToTypedRoundTrip_PreservesConcreteType()
    {
        var payload = new Dictionary<string, object?>
        {
            ["$type"] = "SearchSubmitted",
            ["searchString"] = "docs",
        };

        byte[] bytes = MessagePackSerializer.Serialize(
            payload,
            cancellationToken: TestContext.Current.CancellationToken);
        SearchAction action = MessagePackSerializer.Deserialize<SearchAction>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted typed = Assert.IsType<SearchSubmitted>(action);
        Assert.Equal("docs", typed.SearchString);
    }

    [Fact]
    public void PolymorphicTypedPayload_RoundTripsNestedAndCollectionValues()
    {
        var payload = new SurveyPayload
        {
            PrimaryAction = new SearchSubmitted
            {
                SearchString = "docs",
            },
            Actions = new SearchAction[]
            {
                new SearchSubmitted
                {
                    SearchString = "docs",
                },
                new ValueChanged
                {
                    Value = "updated",
                },
            },
            Question = new ShortTextQuestion
            {
                Label = "What are you looking for?",
            },
        };

        byte[] bytes = MessagePackSerializer.Serialize(
            payload,
            cancellationToken: TestContext.Current.CancellationToken);
        SurveyPayload roundTripped = MessagePackSerializer.Deserialize<SurveyPayload>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted primaryAction = Assert.IsType<SearchSubmitted>(roundTripped.PrimaryAction);
        Assert.Equal("docs", primaryAction.SearchString);
        Assert.Collection(
            roundTripped.Actions,
            action =>
            {
                SearchSubmitted typed = Assert.IsType<SearchSubmitted>(action);
                Assert.Equal("docs", typed.SearchString);
            },
            action =>
            {
                ValueChanged typed = Assert.IsType<ValueChanged>(action);
                Assert.Equal("updated", typed.Value);
            });
        ShortTextQuestion question = Assert.IsType<ShortTextQuestion>(roundTripped.Question);
        Assert.Equal("What are you looking for?", question.Label);
    }



    [Fact]
    public void PolymorphicTypedAction_DeserializesPayloadWhenDiscriminatorIsNotFirstKey()
    {
        byte[] bytes = BuildPolymorphicMapBytes(
            ("searchString", "docs"),
            ("$type", "SearchSubmitted"));

        SearchAction action = MessagePackSerializer.Deserialize<SearchAction>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted typed = Assert.IsType<SearchSubmitted>(action);
        Assert.Equal("docs", typed.SearchString);
    }

    [Fact]
    public void PolymorphicTypedAction_DeserializesPayloadWhenDiscriminatorIsLastKey()
    {
        byte[] bytes = BuildPolymorphicMapBytes(
            ("ignored", "extra"),
            ("searchString", "docs"),
            ("$type", "SearchSubmitted"));

        SearchAction action = MessagePackSerializer.Deserialize<SearchAction>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted typed = Assert.IsType<SearchSubmitted>(action);
        Assert.Equal("docs", typed.SearchString);
    }

    [Fact]
    public void PolymorphicTypedAction_DeserializeIgnoresUnknownKeys()
    {
        byte[] bytes = BuildPolymorphicMapBytes(
            ("$type", "SearchSubmitted"),
            ("searchString", "docs"),
            ("unknownField", "should-be-skipped"));

        SearchAction action = MessagePackSerializer.Deserialize<SearchAction>(
            bytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted typed = Assert.IsType<SearchSubmitted>(action);
        Assert.Equal("docs", typed.SearchString);
    }

    [Fact]
    public void PolymorphicTypedAction_DeserializeThrowsWhenDiscriminatorIsMissing()
    {
        byte[] bytes = BuildPolymorphicMapBytes(
            ("searchString", "docs"));

        MessagePackSerializationException outer = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<SearchAction>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));

        MessagePackSerializationException inner = Assert.IsType<MessagePackSerializationException>(outer.InnerException);
        Assert.Contains("'$type'", inner.Message);
    }

    [Fact]
    public void PolymorphicTypedAction_DeserializeThrowsWhenPayloadIsNotAMap()
    {
        byte[] bytes = MessagePackSerializer.Serialize(
            "not-a-map",
            cancellationToken: TestContext.Current.CancellationToken);

        MessagePackSerializationException outer = Assert.Throws<MessagePackSerializationException>(
            () => MessagePackSerializer.Deserialize<SearchAction>(
                bytes,
                cancellationToken: TestContext.Current.CancellationToken));

        MessagePackSerializationException inner = Assert.IsType<MessagePackSerializationException>(outer.InnerException);
        Assert.Contains("map", inner.Message);
    }

    [Fact]
    public void PolymorphicTypedPayload_RoundTripsNestedPayloadsWhenDiscriminatorIsNotFirst()
    {
        byte[] innerBytes = BuildPolymorphicMapBytes(
            ("searchString", "docs"),
            ("$type", "SearchSubmitted"));
        byte[] outerBytes = BuildMapWithRawEntries(
            ("primaryAction", innerBytes),
            ("actions", MessagePackSerializer.Serialize(
                Array.Empty<SearchAction>(),
                cancellationToken: TestContext.Current.CancellationToken)),
            ("question", MessagePackSerializer.Serialize(
                new Dictionary<string, object?>
                {
                    ["$type"] = "ShortTextQuestion",
                    ["label"] = "Pick one",
                },
                cancellationToken: TestContext.Current.CancellationToken)));

        SurveyPayload payload = MessagePackSerializer.Deserialize<SurveyPayload>(
            outerBytes,
            cancellationToken: TestContext.Current.CancellationToken);

        SearchSubmitted primary = Assert.IsType<SearchSubmitted>(payload.PrimaryAction);
        Assert.Equal("docs", primary.SearchString);
        Assert.Empty(payload.Actions);
        ShortTextQuestion question = Assert.IsType<ShortTextQuestion>(payload.Question);
        Assert.Equal("Pick one", question.Label);
    }

    private static byte[] BuildPolymorphicMapBytes(params (string Key, string Value)[] entries)
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

    private static byte[] BuildMapWithRawEntries(params (string Key, byte[] ValueBytes)[] entries)
    {
        ArrayBufferWriter<byte> buffer = new();
        MessagePackWriter writer = new(buffer);
        writer.WriteMapHeader(entries.Length);
        foreach ((string key, byte[] valueBytes) in entries)
        {
            writer.Write(key);
            writer.WriteRaw(valueBytes);
        }
        writer.Flush();
        return buffer.WrittenSpan.ToArray();
    }
























}
