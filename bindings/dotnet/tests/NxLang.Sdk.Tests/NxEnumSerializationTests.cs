// Copyright (c) Bret Johnson. All rights reserved.
// Licensed under the MIT license. See LICENSE file in the project root for full license information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using NxLang.Nx;
using NxLang.Nx.Serialization;
using Xunit;

namespace NxLang.Nx.Tests;

[JsonConverter(typeof(NxEnumJsonConverter<TestDealStage, TestDealStageWireFormat>))]
[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<TestDealStage, TestDealStageWireFormat>))]
internal enum TestDealStage
{
    Draft,
    PendingReview,
    ClosedWon,
}

internal sealed class TestDealStageWireFormat : INxEnumWireFormat<TestDealStage>
{
    public static string Format(TestDealStage value)
    {
        return value switch
        {
            TestDealStage.Draft => "draft",
            TestDealStage.PendingReview => "pending_review",
            TestDealStage.ClosedWon => "closed_won",
            _ => throw new FormatException("Unknown NX enum value."),
        };
    }

    public static TestDealStage Parse(string value)
    {
        return value switch
        {
            "draft" => TestDealStage.Draft,
            "pending_review" => TestDealStage.PendingReview,
            "closed_won" => TestDealStage.ClosedWon,
            _ => throw new FormatException("Unknown NX enum member."),
        };
    }
}

public class NxEnumSerializationTests
{
    [Fact]
    public void SharedEnumHelperTypes_ArePublic()
    {
        Assert.True(typeof(INxEnumWireFormat<>).IsPublic);
        Assert.True(typeof(NxEnumJsonConverter<,>).IsPublic);
        Assert.True(typeof(NxEnumMessagePackFormatter<,>).IsPublic);
    }

    [Fact]
    public void GeneratedStyleJsonEnum_RoundTripsAuthoredMemberString()
    {
        string json = JsonSerializer.Serialize(TestDealStage.PendingReview);

        Assert.Equal("\"pending_review\"", json);
        Assert.Equal(
            TestDealStage.PendingReview,
            JsonSerializer.Deserialize<TestDealStage>(json));
    }

    [Fact]
    public void GeneratedStyleMessagePackEnum_RoundTripsAuthoredMemberString()
    {
        byte[] payload = MessagePackSerializer.Serialize(
            TestDealStage.PendingReview,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(
            "pending_review",
            MessagePackSerializer.Deserialize<string>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(
            TestDealStage.PendingReview,
            MessagePackSerializer.Deserialize<TestDealStage>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
    }

    [Fact]
    public void NxSeverity_RoundTripsThroughSharedEnumHelpers()
    {
        string json = JsonSerializer.Serialize(NxSeverity.Warning);
        byte[] payload = MessagePackSerializer.Serialize(
            NxSeverity.Warning,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("\"warning\"", json);
        Assert.Equal(NxSeverity.Warning, JsonSerializer.Deserialize<NxSeverity>(json));
        Assert.Equal(
            "warning",
            MessagePackSerializer.Deserialize<string>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
        Assert.Equal(
            NxSeverity.Warning,
            MessagePackSerializer.Deserialize<NxSeverity>(
                payload,
                cancellationToken: TestContext.Current.CancellationToken));
    }

    [Fact]
    public void SharedEnumJsonHelpers_RejectNonStringTokens()
    {
        AssertJsonEnumDeserializationFails<TestDealStage>(
            "123",
            "Expected NX enum to be encoded as a JSON string.");
        AssertJsonEnumDeserializationFails<NxSeverity>(
            "123",
            "Expected NX enum to be encoded as a JSON string.");
    }

    [Fact]
    public void SharedEnumJsonHelpers_RejectUnknownMembers()
    {
        AssertJsonEnumDeserializationFails<TestDealStage>(
            "\"mystery\"",
            "Unknown NX enum member.",
            expectFormatExceptionInner: true);
        AssertJsonEnumDeserializationFails<NxSeverity>(
            "\"mystery\"",
            "Unknown NX severity value.",
            expectFormatExceptionInner: true);
    }

    [Fact]
    public void SharedEnumMessagePackHelpers_RejectNilTokens()
    {
        byte[] payload = MessagePackSerializer.Serialize<string?>(
            null,
            cancellationToken: TestContext.Current.CancellationToken);

        AssertMessagePackEnumDeserializationFails<TestDealStage>(
            payload,
            "Expected NX enum to be encoded as a MessagePack string.");
        AssertMessagePackEnumDeserializationFails<NxSeverity>(
            payload,
            "Expected NX enum to be encoded as a MessagePack string.");
    }

    [Fact]
    public void SharedEnumMessagePackHelpers_RejectUnknownMembers()
    {
        byte[] payload = MessagePackSerializer.Serialize(
            "mystery",
            cancellationToken: TestContext.Current.CancellationToken);

        AssertMessagePackEnumDeserializationFails<TestDealStage>(
            payload,
            "Unknown NX enum member.",
            expectFormatExceptionInner: true);
        AssertMessagePackEnumDeserializationFails<NxSeverity>(
            payload,
            "Unknown NX severity value.",
            expectFormatExceptionInner: true);
    }

    private static void AssertJsonEnumDeserializationFails<TEnum>(
        string json,
        string expectedMessage,
        bool expectFormatExceptionInner = false)
        where TEnum : struct, Enum
    {
        JsonException exception = Assert.Throws<JsonException>(() => JsonSerializer.Deserialize<TEnum>(json));

        Assert.Contains(expectedMessage, exception.Message);

        if (expectFormatExceptionInner)
        {
            Assert.IsType<FormatException>(exception.InnerException);
        }
        else
        {
            Assert.Null(exception.InnerException);
        }
    }

    private static void AssertMessagePackEnumDeserializationFails<TEnum>(
        byte[] payload,
        string expectedMessage,
        bool expectFormatExceptionInner = false)
        where TEnum : struct, Enum
    {
        MessagePackSerializationException exception =
            Assert.Throws<MessagePackSerializationException>(
                () => MessagePackSerializer.Deserialize<TEnum>(
                    payload,
                    cancellationToken: TestContext.Current.CancellationToken));

        Assert.Contains($"Failed to deserialize {typeof(TEnum).FullName}", exception.Message);

        MessagePackSerializationException innerException =
            Assert.IsType<MessagePackSerializationException>(exception.InnerException);

        Assert.Equal(expectedMessage, innerException.Message);

        if (expectFormatExceptionInner)
        {
            Assert.IsType<FormatException>(innerException.InnerException);
        }
        else
        {
            Assert.Null(innerException.InnerException);
        }
    }
}
